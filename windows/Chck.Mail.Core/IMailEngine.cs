namespace Chck.Mail.Core;

public interface IMailEngine
{
    Task<IReadOnlyList<MailProvider>> ListProvidersAsync(CancellationToken ct = default);
    Task<Account> AddAccountAsync(AddAccountRequest request, CancellationToken ct = default);
    Task<IReadOnlyList<Account>> ListAccountsAsync(CancellationToken ct = default);
    Task<AccountSettings> GetAccountSettingsAsync(string accountId, CancellationToken ct = default);
    Task<Account> UpdateAccountAsync(string accountId, UpdateAccountRequest request, CancellationToken ct = default);
    Task RemoveAccountAsync(string accountId, CancellationToken ct = default);
    Task<ConnectProbe> TestAccountAsync(AddAccountRequest request, CancellationToken ct = default);
    Task<IReadOnlyList<Folder>> ListFoldersAsync(string accountId, CancellationToken ct = default);
    Task<IReadOnlyList<Folder>> ListCachedFoldersAsync(string accountId, CancellationToken ct = default) => ListFoldersAsync(accountId, ct);
    Task MarkReadAsync(string messageId, CancellationToken ct = default) => SetFlagsAsync(messageId, new MailFlags(Seen: true), ct);
    Task SyncFolderAsync(string folderId, CancellationToken ct = default);
    Task TickAsync(string? accountId = null, bool idle = false, CancellationToken ct = default);
    Task<IReadOnlyList<MessageRow>> ListMessagesAsync(string folderId, CancellationToken ct = default);
    Task<IReadOnlyList<MessageRow>> UnifiedInboxAsync(CancellationToken ct = default);
    Task<MessageBody?> GetCachedBodyAsync(string messageId, CancellationToken ct = default) => Task.FromResult<MessageBody?>(null);
    Task<int> ClearFailedQueueAsync(CancellationToken ct = default) => Task.FromResult(0);
    Task<MessageBody> GetBodyAsync(string messageId, CancellationToken ct = default);
    Task SetCachedFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default) => Task.CompletedTask;
    Task SetFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default);
    Task DeleteMessageAsync(string messageId, CancellationToken ct = default);
    Task<string> SendAsync(SendRequest request, CancellationToken ct = default);
    Task UndoSendAsync(string draftId, CancellationToken ct = default);
    Task<uint> FlushDueSendsAsync(CancellationToken ct = default);
    Task<IReadOnlyList<MessageRow>> SearchAsync(string query, CancellationToken ct = default);
    Task<IReadOnlyList<AttachmentHandle>> ListAttachmentsAsync(string messageId, CancellationToken ct = default);
}
