using System.Collections.Concurrent;
using Chck.Mail.Core;

namespace Chck.Mail.Tests;

internal sealed class ControlledMailEngine : IMailEngine
{
    public IReadOnlyList<Account> Accounts { get; set; } = [new("a", "a@imyemail.test", "A", "custom"), new("b", "b@imyemail.test", "B", "custom")];
    public Dictionary<string, IReadOnlyList<MessageRow>> Rows { get; } = new()
    {
        ["a:INBOX"] = [],
        ["b:INBOX"] = [],
        ["a:Archive"] = [],
    };
    public Dictionary<string, MessageBody> Cached { get; } = new();
    public ConcurrentDictionary<string, int> Fetches { get; } = new();
    public ConcurrentBag<string> Synced { get; } = [];
    public Func<string, Task<MessageBody>>? Fetch { get; set; }
    public Func<string, Task<IReadOnlyList<AttachmentHandle>>>? Attachments { get; set; }
    public Func<string, Task>? Sync { get; set; }
    public Func<string, Task>? MarkRead { get; set; }
    public IReadOnlyList<MessageRow> SearchRows { get; set; } = [];
    public int CacheReads;
    public int FlagWrites;
    public int ClearCalls;
    public ConcurrentBag<SendRequest> Sent { get; } = [];
    public Func<SendRequest, Task<string>>? Send { get; set; }

    public static MessageRow Row(string id, bool unread = true, long date = 1) => new(id, id, "sender@imyemail.test", "snippet", unread, date);
    public Task<IReadOnlyList<MailProvider>> ListProvidersAsync(CancellationToken ct = default) => Task.FromResult<IReadOnlyList<MailProvider>>([]);
    public Task<IReadOnlyList<Account>> ListAccountsAsync(CancellationToken ct = default) => Task.FromResult(Accounts);
    public Task<IReadOnlyList<Folder>> ListFoldersAsync(string accountId, CancellationToken ct = default) => Task.FromResult<IReadOnlyList<Folder>>(
        Rows.Where(pair => pair.Key.StartsWith(accountId + ":", StringComparison.Ordinal))
            .Select(pair => new Folder(pair.Key, pair.Key[(accountId.Length + 1)..], pair.Key.EndsWith(":INBOX", StringComparison.Ordinal) ? "inbox" : "archive", (uint)pair.Value.Count(row => row.Unread))).ToArray());
    public async Task SyncFolderAsync(string folderId, CancellationToken ct = default)
    {
        Synced.Add(folderId);
        if (Sync is not null) await Sync(folderId);
    }
    public Task<IReadOnlyList<MessageRow>> ListMessagesAsync(string folderId, CancellationToken ct = default) => Task.FromResult(Rows.GetValueOrDefault(folderId) ?? []);
    public Task<IReadOnlyList<MessageRow>> UnifiedInboxAsync(CancellationToken ct = default) => Task.FromResult<IReadOnlyList<MessageRow>>(
        Rows.Where(pair => pair.Key.EndsWith(":INBOX", StringComparison.Ordinal)).SelectMany(pair => pair.Value).OrderByDescending(row => row.DateUnix).ToArray());
    public Task<MessageBody?> GetCachedBodyAsync(string messageId, CancellationToken ct = default)
    {
        Interlocked.Increment(ref CacheReads);
        return Task.FromResult(Cached.GetValueOrDefault(messageId));
    }
    public Task<MessageBody> GetBodyAsync(string messageId, CancellationToken ct = default)
    {
        Fetches.AddOrUpdate(messageId, 1, (_, count) => count + 1);
        return Fetch?.Invoke(messageId) ?? Task.FromResult(new MessageBody(messageId, ""));
    }
    public async Task MarkReadAsync(string messageId, CancellationToken ct = default)
    {
        Interlocked.Increment(ref FlagWrites);
        if (MarkRead is not null) await MarkRead(messageId);
        foreach (var key in Rows.Keys.ToArray())
            Rows[key] = Rows[key].Select(row => row.Id == messageId ? row with { Unread = false } : row).ToArray();
    }
    public Task SetFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default)
    {
        Interlocked.Increment(ref FlagWrites);
        return Task.CompletedTask;
    }
    public Task<IReadOnlyList<AttachmentHandle>> ListAttachmentsAsync(string messageId, CancellationToken ct = default)
        => Attachments?.Invoke(messageId) ?? Task.FromResult<IReadOnlyList<AttachmentHandle>>([]);
    public Task<IReadOnlyList<MessageRow>> SearchAsync(string query, CancellationToken ct = default) => Task.FromResult(SearchRows);
    public Task<int> ClearFailedQueueAsync(CancellationToken ct = default)
    {
        ClearCalls++;
        return Task.FromResult(3);
    }
    public Task<uint> FlushDueSendsAsync(CancellationToken ct = default) => Task.FromResult(0u);
    public Task TickAsync(string? accountId = null, bool idle = false, CancellationToken ct = default) => Task.CompletedTask;
    public Task DeleteMessageAsync(string messageId, CancellationToken ct = default) => Task.CompletedTask;
    public Task UndoSendAsync(string draftId, CancellationToken ct = default) => Task.CompletedTask;
    public Task RemoveAccountAsync(string accountId, CancellationToken ct = default)
    {
        Accounts = Accounts.Where(a => a.Id != accountId).ToArray();
        foreach (var key in Rows.Keys.Where(k => k.StartsWith(accountId + ":", StringComparison.Ordinal)).ToArray()) Rows.Remove(key);
        return Task.CompletedTask;
    }
    public Task<AccountSettings> GetAccountSettingsAsync(string accountId, CancellationToken ct = default)
    {
        var account = Accounts.FirstOrDefault(a => a.Id == accountId) ?? throw EngineException.NotFound();
        return Task.FromResult(new AccountSettings(accountId, account.Email, account.DisplayName, account.Email,
            "imap.imyemail.test", 993, false, "smtp.imyemail.test", 465, false));
    }
    public Task<Account> UpdateAccountAsync(string accountId, UpdateAccountRequest request, CancellationToken ct = default)
    {
        var account = Accounts.FirstOrDefault(a => a.Id == accountId) ?? throw EngineException.NotFound();
        account = account with { DisplayName = request.DisplayName };
        Accounts = Accounts.Select(a => a.Id == accountId ? account : a).ToArray();
        return Task.FromResult(account);
    }
    public Task<Account> AddAccountAsync(AddAccountRequest request, CancellationToken ct = default) => throw new NotSupportedException();
    public Task<ConnectProbe> TestAccountAsync(AddAccountRequest request, CancellationToken ct = default) => throw new NotSupportedException();
    public Task<string> SendAsync(SendRequest request, CancellationToken ct = default)
    {
        Sent.Add(request);
        return Send?.Invoke(request) ?? Task.FromResult("draft");
    }
}
