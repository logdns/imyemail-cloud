using System.Text;

namespace Chck.Mail.Core;

/// <summary>Thread-safe bounded body cache keyed by the complete account-qualified message ID.</summary>
public sealed class MailBodyCache(IMailEngine engine, int capacity = 24, int byteLimit = 8 * 1024 * 1024)
{
    private sealed record Entry(MessageBody Body, long Bytes, LinkedListNode<string> Node);
    private readonly object _gate = new();
    private readonly Dictionary<string, Entry> _entries = new(StringComparer.Ordinal);
    private readonly LinkedList<string> _lru = new();
    private readonly Dictionary<string, TaskCompletionSource<MessageBody>> _pending = new(StringComparer.Ordinal);
    private long _bytes;

    public MessageBody? Peek(string id)
    {
        lock (_gate)
        {
            if (!_entries.TryGetValue(id, out var entry)) return null;
            _lru.Remove(entry.Node);
            _lru.AddLast(entry.Node);
            return entry.Body;
        }
    }

    public Task<MessageBody> LoadAsync(string id, CancellationToken ct = default)
    {
        if (ct.IsCancellationRequested) return Task.FromCanceled<MessageBody>(ct);
        TaskCompletionSource<MessageBody> request;
        lock (_gate)
        {
            var hit = Peek(id);
            if (hit is not null) return Task.FromResult(hit);
            if (_pending.TryGetValue(id, out request!)) return request.Task.WaitAsync(ct);
            request = new(TaskCreationOptions.RunContinuationsAsynchronously);
            _pending[id] = request;
        }
        _ = Task.Run(() => FetchAsync(id, request));
        return request.Task.WaitAsync(ct);
    }

    private async Task FetchAsync(string id, TaskCompletionSource<MessageBody> request)
    {
        try
        {
            var body = await engine.GetCachedBodyAsync(id).ConfigureAwait(false)
                ?? await engine.GetBodyAsync(id).ConfigureAwait(false);
            lock (_gate)
            {
                // The request object is its generation: invalidation/replacement prevents late refill.
                if (_pending.TryGetValue(id, out var current) && ReferenceEquals(current, request))
                {
                    _pending.Remove(id);
                    Insert(id, body);
                }
            }
            request.TrySetResult(body);
        }
        catch (Exception ex)
        {
            lock (_gate)
            {
                if (_pending.TryGetValue(id, out var current) && ReferenceEquals(current, request)) _pending.Remove(id);
            }
            request.TrySetException(ex);
        }
    }

    public void Remove(string id)
    {
        lock (_gate)
        {
            Evict(id);
            _pending.Remove(id);
        }
    }

    public void Clear()
    {
        lock (_gate)
        {
            _entries.Clear();
            _lru.Clear();
            _pending.Clear();
            _bytes = 0;
        }
    }

    private void Insert(string id, MessageBody body)
    {
        long cost = (long)Encoding.UTF8.GetByteCount(id) + Encoding.UTF8.GetByteCount(body.Text) + Encoding.UTF8.GetByteCount(body.Html);
        if (capacity <= 0 || byteLimit <= 0 || cost > byteLimit) return;
        Evict(id);
        while (_entries.Count >= capacity || _bytes + cost > byteLimit)
        {
            if (_lru.First is null) break;
            Evict(_lru.First.Value);
        }
        _entries[id] = new(body, cost, _lru.AddLast(id));
        _bytes += cost;
    }

    private void Evict(string id)
    {
        if (!_entries.Remove(id, out var entry)) return;
        _bytes -= entry.Bytes;
        _lru.Remove(entry.Node);
    }
}
