namespace Chck.Mail.Core;

public sealed class PreviewMailEngine : IMailEngine
{
    private readonly Dictionary<string, AccountSettings> _settings = new();
    private readonly List<Account> _accounts =
    [
        new("preview", "dev@imyemail.test", "Dev", "custom"),
    ];

    public Task<IReadOnlyList<MailProvider>> ListProvidersAsync(CancellationToken ct = default)
    {
        var catalog = ProviderCatalog.Builtin();
        return Task.FromResult(catalog.Providers);
    }

    public Task<Account> AddAccountAsync(AddAccountRequest request, CancellationToken ct = default)
    {
        if (!request.Email.Contains('@', StringComparison.Ordinal))
        {
            throw EngineException.Invalid("email");
        }

        var provider = ProviderCatalog.Builtin().ByEmail(request.Email);
        var acc = new Account(Guid.NewGuid().ToString("N"), request.Email, request.DisplayName ?? request.Email.Split('@')[0], provider?.Id ?? "custom");
        _accounts.Add(acc);
        var config = ProviderCatalog.WithDefaults(request, provider);
        _settings[acc.Id] = new(acc.Id, acc.Email, acc.DisplayName, config.Username ?? acc.Email,
            config.ImapHost ?? "", config.ImapPort ?? 993, config.ImapStartTls,
            config.SmtpHost ?? "", config.SmtpPort ?? 465, config.SmtpStartTls);
        return Task.FromResult(acc);
    }

    public Task<IReadOnlyList<Account>> ListAccountsAsync(CancellationToken ct = default)
        => Task.FromResult<IReadOnlyList<Account>>(_accounts.ToList());

    public Task RemoveAccountAsync(string accountId, CancellationToken ct = default)
    {
        _accounts.RemoveAll(a => a.Id == accountId);
        _settings.Remove(accountId);
        return Task.CompletedTask;
    }

    public Task<AccountSettings> GetAccountSettingsAsync(string accountId, CancellationToken ct = default)
    {
        var account = _accounts.FirstOrDefault(a => a.Id == accountId) ?? throw EngineException.NotFound();
        return Task.FromResult(_settings.GetValueOrDefault(accountId) ?? new(accountId, account.Email,
            account.DisplayName, account.Email, "imap.imyemail.test", 993, false, "smtp.imyemail.test", 465, false));
    }

    public Task<Account> UpdateAccountAsync(string accountId, UpdateAccountRequest request, CancellationToken ct = default)
    {
        var index = _accounts.FindIndex(a => a.Id == accountId);
        if (index < 0) throw EngineException.NotFound();
        var account = _accounts[index] with { DisplayName = request.DisplayName };
        _accounts[index] = account;
        _settings[accountId] = new(accountId, account.Email, account.DisplayName, request.Username,
            request.ImapHost, request.ImapPort, request.ImapStartTls, request.SmtpHost, request.SmtpPort, request.SmtpStartTls);
        return Task.FromResult(account);
    }

    public Task<ConnectProbe> TestAccountAsync(AddAccountRequest request, CancellationToken ct = default)
        => Task.FromResult(new ConnectProbe([new ProbeStep("dns", "ok", "preview")]));

    public Task<IReadOnlyList<Folder>> ListFoldersAsync(string accountId, CancellationToken ct = default)
        => Task.FromResult<IReadOnlyList<Folder>>([new($"{accountId}:INBOX", "INBOX", "inbox", 1)]);

    public Task SyncFolderAsync(string folderId, CancellationToken ct = default) => Task.CompletedTask;

    public Task TickAsync(string? accountId = null, bool idle = false, CancellationToken ct = default)
        => Task.CompletedTask;

    public Task<IReadOnlyList<MessageRow>> ListMessagesAsync(string folderId, CancellationToken ct = default)
        => Task.FromResult<IReadOnlyList<MessageRow>>([
            new($"{folderId}:1", "Hello from testkit", "alice@imyemail.test", "Hello", true),
        ]);

    public Task<IReadOnlyList<MessageRow>> UnifiedInboxAsync(CancellationToken ct = default)
        => ListMessagesAsync("preview:INBOX", ct);

    public Task<MessageBody> GetBodyAsync(string messageId, CancellationToken ct = default)
        => Task.FromResult(new MessageBody("Hello from testkit", "<p>Hello from testkit</p>"));

    public Task SetFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default) => Task.CompletedTask;

    public Task DeleteMessageAsync(string messageId, CancellationToken ct = default) => Task.CompletedTask;

    public Task<string> SendAsync(SendRequest request, CancellationToken ct = default)
        => Task.FromResult("preview-draft");

    public Task UndoSendAsync(string draftId, CancellationToken ct = default) => Task.CompletedTask;

    public Task<uint> FlushDueSendsAsync(CancellationToken ct = default) => Task.FromResult(0u);

    public Task<IReadOnlyList<MessageRow>> SearchAsync(string query, CancellationToken ct = default)
        => ListMessagesAsync("preview:INBOX", ct);

    public Task<IReadOnlyList<AttachmentHandle>> ListAttachmentsAsync(string messageId, CancellationToken ct = default)
        => Task.FromResult<IReadOnlyList<AttachmentHandle>>([]);
}
