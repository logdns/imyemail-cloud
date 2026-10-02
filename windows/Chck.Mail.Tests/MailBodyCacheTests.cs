using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class MailBodyCacheTests
{
    [Fact]
    public async Task DiskAndMemoryHitsNeverFetchOrChangeFlags()
    {
        var engine = new ControlledMailEngine();
        engine.Cached["a:INBOX:1"] = new("stored", "");
        var cache = new MailBodyCache(engine);
        Assert.Equal("stored", (await cache.LoadAsync("a:INBOX:1")).Text);
        Assert.Equal("stored", (await cache.LoadAsync("a:INBOX:1")).Text);
        Assert.Equal(1, engine.CacheReads);
        Assert.Empty(engine.Fetches);
        Assert.Equal(0, engine.FlagWrites);
    }

    [Fact]
    public async Task ConcurrentRequestsCoalesceAndOneCallerCancellationDoesNotCancelOthers()
    {
        var engine = new ControlledMailEngine();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var response = new TaskCompletionSource<MessageBody>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Fetch = _ => { entered.SetResult(); return response.Task; };
        var cache = new MailBodyCache(engine);
        using var cancellation = new CancellationTokenSource();
        var first = cache.LoadAsync("a:INBOX:1", cancellation.Token);
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        var second = cache.LoadAsync("a:INBOX:1");
        cancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => first);
        response.SetResult(new("shared", ""));
        Assert.Equal("shared", (await second).Text);
        Assert.Equal(1, engine.Fetches["a:INBOX:1"]);
        Assert.Equal("shared", cache.Peek("a:INBOX:1")?.Text);
    }

    [Fact]
    public async Task AccountQualifiedKeysAndLruEviction()
    {
        var cache = new MailBodyCache(new ControlledMailEngine(), capacity: 2);
        _ = await cache.LoadAsync("a:INBOX:1");
        _ = await cache.LoadAsync("b:INBOX:1");
        Assert.NotNull(cache.Peek("a:INBOX:1"));
        _ = await cache.LoadAsync("a:INBOX:2");
        Assert.Null(cache.Peek("b:INBOX:1"));
        Assert.Equal("a:INBOX:1", cache.Peek("a:INBOX:1")?.Text);
    }

    [Fact]
    public async Task Utf8BudgetAndOversizedBodiesAreBounded()
    {
        var engine = new ControlledMailEngine();
        engine.Cached["a"] = new("你好", "");
        engine.Cached["b"] = new("世界", "");
        engine.Cached["large"] = new(new string('x', 20), "");
        var cache = new MailBodyCache(engine, byteLimit: 10);
        _ = await cache.LoadAsync("a");
        _ = await cache.LoadAsync("b");
        Assert.Null(cache.Peek("a"));
        _ = await cache.LoadAsync("large");
        Assert.Null(cache.Peek("large"));
        Assert.NotNull(cache.Peek("b"));
    }

    [Fact]
    public async Task FailedRequestCanRetry()
    {
        var engine = new ControlledMailEngine();
        engine.Fetch = id => engine.Fetches[id] == 1 ? Task.FromException<MessageBody>(new IOException("offline")) : Task.FromResult(new MessageBody("retried", ""));
        var cache = new MailBodyCache(engine);
        await Assert.ThrowsAsync<IOException>(() => cache.LoadAsync("a:INBOX:1"));
        Assert.Null(cache.Peek("a:INBOX:1"));
        Assert.Equal("retried", (await cache.LoadAsync("a:INBOX:1")).Text);
    }

    [Fact]
    public async Task InvalidationPreventsLateResponseFromOverwritingReplacement()
    {
        var engine = new ControlledMailEngine();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var response = new TaskCompletionSource<MessageBody>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Fetch = id =>
        {
            if (engine.Fetches[id] != 1) return Task.FromResult(new MessageBody("new", ""));
            entered.SetResult();
            return response.Task;
        };
        var cache = new MailBodyCache(engine);
        var old = cache.LoadAsync("a:INBOX:1");
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        cache.Remove("a:INBOX:1");
        _ = await cache.LoadAsync("a:INBOX:1");
        response.SetResult(new("old", ""));
        _ = await old;
        Assert.Equal("new", cache.Peek("a:INBOX:1")?.Text);
        cache.Clear();
        Assert.Null(cache.Peek("a:INBOX:1"));
    }
    [Fact]
    public async Task ClearWhileFetchingDoesNotRestoreRemovedBody()
    {
        var engine = new ControlledMailEngine();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var response = new TaskCompletionSource<MessageBody>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Fetch = _ => { entered.SetResult(); return response.Task; };
        var cache = new MailBodyCache(engine);
        var pending = cache.LoadAsync("a:INBOX:1");
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        cache.Clear();
        response.SetResult(new("late", ""));
        _ = await pending;
        Assert.Null(cache.Peek("a:INBOX:1"));
    }

}
