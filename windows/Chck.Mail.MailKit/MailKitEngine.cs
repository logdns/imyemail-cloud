using System.Linq;
using System.Net.Security;
using System.Text.Json;
using Chck.Mail.Core;
using Chck.Mail.Data;
using MailKit;
using Store = Chck.Mail.Data.MailStore;
using MailKit.Net.Imap;
using MailKit.Net.Smtp;
using MailKit.Security;
using MimeKit;
using Microsoft.Data.Sqlite;

namespace Chck.Mail.MailKit;

public sealed class MailKitEngine : IMailEngine, IDisposable
{
    private readonly Store _store;
    private readonly ISecretStore _secrets;
    private readonly ProviderCatalog _catalog;
    private readonly object _gate = new();
    private readonly SemaphoreSlim _operations = new(1, 1);
    private readonly CancellationTokenSource _lifetime = new();
    private readonly string? _diskPath;
    private int _disposeRequested;

    public MailKitEngine(string dbPath)
        : this(dbPath, OpenOsSecrets(dbPath))
    {
    }

    public MailKitEngine(string dbPath, ISecretStore secrets)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(dbPath)) ?? ".");
        _store = new Store(dbPath);
        _diskPath = dbPath == ":memory:" ? null : Path.GetFullPath(dbPath);
        _secrets = secrets;
        _catalog = ProviderCatalog.Builtin();
    }

    private MailKitEngine(Store store, ISecretStore secrets)
    {
        _store = store;
        _secrets = secrets;
        _catalog = ProviderCatalog.Builtin();
    }

    public static MailKitEngine OpenInMemory() => new(Store.OpenInMemory(), SecretStore.Memory());

    public static ISecretStore OpenOsSecrets(string dbPath)
    {
        var sidecar = SecretStore.OpenBeside(dbPath);
        if (OperatingSystem.IsWindows())
        {
            return new OverlaySecretStore(new WindowsCredentialStore(), sidecar);
        }

        return sidecar;
    }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposeRequested, 1) != 0) return;
        _lifetime.Cancel();
        lock (_gate) { _store.Dispose(); }
    }

    // MailStore owns one SqliteConnection. All normal operations use this lane;
    // no connection access escapes its lock, including synchronous helper calls.
    private async Task<T> RunSerialized<T>(Func<CancellationToken, T> operation, CancellationToken ct)
    {
        ObjectDisposedException.ThrowIf(Volatile.Read(ref _disposeRequested) != 0, this);
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(ct, _lifetime.Token);
        await _operations.WaitAsync(linked.Token).ConfigureAwait(false);
        try
        {
            return await Task.Run(() =>
            {
                lock (_gate)
                {
                    linked.Token.ThrowIfCancellationRequested();
                    var result = operation(linked.Token);
                    linked.Token.ThrowIfCancellationRequested();
                    return result;
                }
            }, linked.Token).ConfigureAwait(false);
        }
        finally { _operations.Release(); }
    }

    private Task RunSerialized(Action<CancellationToken> operation, CancellationToken ct)
        => RunSerialized(token => { operation(token); return true; }, ct);

    private Task<T> ReadLocal<T>(Func<Store, T> read, CancellationToken ct)
    {
        if (_diskPath is null) return RunSerialized(_ => read(_store), ct);
        return Task.Run(() =>
        {
            ct.ThrowIfCancellationRequested();
            ObjectDisposedException.ThrowIf(Volatile.Read(ref _disposeRequested) != 0, this);
            using var store = Store.OpenReadOnly(_diskPath);
            var value = read(store);
            ct.ThrowIfCancellationRequested();
            return value;
        }, ct);
    }

    public Task<IReadOnlyList<Folder>> ListCachedFoldersAsync(string accountId, CancellationToken ct = default)
        => ReadLocal(store => store.ListFolders(accountId), ct);

    public async Task MarkReadAsync(string messageId, CancellationToken ct = default)
    {
        await RunSerialized(_ =>
        {
            var flags = _store.GetMessageFlags(messageId) ?? throw EngineException.NotFound();
            if (flags.Seen) return;
            var meta = _store.GetMessageMeta(messageId) ?? throw EngineException.NotFound();
            var updated = flags with { Seen = true };
            _store.SetMessageFlags(messageId, updated);
            _store.EnqueueOp(meta.AccountId, "mark_read", JsonSerializer.Serialize(new
            {
                id = messageId,
                uid = meta.Uid,
                source = meta.FolderPath,
                uidvalidity = _store.GetFolderUidValidity(meta.FolderId),
                flags = updated.ToBits(),
            }));
        }, ct).ConfigureAwait(false);
    }

    public Task<IReadOnlyList<MailProvider>> ListProvidersAsync(CancellationToken ct = default)
        => Task.FromResult<IReadOnlyList<MailProvider>>(_catalog.Providers);
    public Task<Account> AddAccountAsync(AddAccountRequest request, CancellationToken ct = default)
        => RunSerialized(token => AddAccount(request, token), ct);
    public Task<IReadOnlyList<Account>> ListAccountsAsync(CancellationToken ct = default)
        => ReadLocal(store => store.ListAccounts(), ct);
    public Task<AccountSettings> GetAccountSettingsAsync(string accountId, CancellationToken ct = default)
        => RunSerialized(_ => GetAccountSettings(accountId), ct);
    public Task<Account> UpdateAccountAsync(string accountId, UpdateAccountRequest request, CancellationToken ct = default)
        => RunSerialized(_ => UpdateAccount(accountId, request), ct);
    public Task RemoveAccountAsync(string accountId, CancellationToken ct = default)
        => RunSerialized(token => RemoveAccount(accountId, token), ct);
    public Task<ConnectProbe> TestAccountAsync(AddAccountRequest request, CancellationToken ct = default)
        => RunSerialized(token => TestAccount(request, token), ct);
    public Task<IReadOnlyList<Folder>> ListFoldersAsync(string accountId, CancellationToken ct = default)
        => RunSerialized(token => ListFolders(accountId, token), ct);
    public Task SyncFolderAsync(string folderId, CancellationToken ct = default)
        => RunSerialized(token => SyncFolder(folderId, token), ct);
    public Task TickAsync(string? accountId = null, bool idle = false, CancellationToken ct = default)
        => RunSerialized(token => Tick(accountId, idle, token), ct);
    public Task<IReadOnlyList<MessageRow>> ListMessagesAsync(string folderId, CancellationToken ct = default)
        => ReadLocal(store => store.ListMessages(folderId), ct);
    public Task<IReadOnlyList<MessageRow>> UnifiedInboxAsync(CancellationToken ct = default)
        => ReadLocal(store => store.UnifiedInbox(), ct);
    public async Task<MessageBody> GetBodyAsync(string messageId, CancellationToken ct = default)
        => await GetCachedBodyAsync(messageId, ct).ConfigureAwait(false)
            ?? await RunSerialized(token => GetBody(messageId, token), ct).ConfigureAwait(false);
    public Task SetCachedFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default)
        => RunSerialized(_ => _store.SetMessageFlags(messageId, flags), ct);
    public Task SetFlagsAsync(string messageId, MailFlags flags, CancellationToken ct = default)
        => RunSerialized(token => SetFlags(messageId, flags, token), ct);
    public Task DeleteMessageAsync(string messageId, CancellationToken ct = default)
        => RunSerialized(token => DeleteMessage(messageId, token), ct);
    public Task<string> SendAsync(SendRequest request, CancellationToken ct = default)
        => RunSerialized(token => Send(request, token), ct);
    public Task UndoSendAsync(string draftId, CancellationToken ct = default)
        => RunSerialized(token => UndoSend(draftId, token), ct);
    public Task<uint> FlushDueSendsAsync(CancellationToken ct = default)
        => RunSerialized(FlushDueSends, ct);
    public Task<IReadOnlyList<MessageRow>> SearchAsync(string query, CancellationToken ct = default)
        => ReadLocal(store => store.Search(query), ct);
    public Task<IReadOnlyList<AttachmentHandle>> ListAttachmentsAsync(string messageId, CancellationToken ct = default)
        => ReadLocal(store => store.ListAttachments(messageId), ct);
    public Task<int> ClearFailedQueueAsync(CancellationToken ct = default)
        => RunSerialized(_ => _store.ClearFailedQueue(), ct);

    public Task<MessageBody?> GetCachedBodyAsync(string messageId, CancellationToken ct = default)
    {
        if (_diskPath is null) return RunSerialized(_ => _store.GetBody(messageId), ct);
        return Task.Run(() =>
        {
            ct.ThrowIfCancellationRequested();
            ObjectDisposedException.ThrowIf(Volatile.Read(ref _disposeRequested) != 0, this);
            // An independent read-only connection avoids the network lane and
            // never runs migrations or marks the message read.
            using var connection = new SqliteConnection(new SqliteConnectionStringBuilder
            {
                DataSource = _diskPath,
                Mode = SqliteOpenMode.ReadOnly,
                Cache = SqliteCacheMode.Private,
                Pooling = false,
                DefaultTimeout = 1,
            }.ToString());
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "SELECT html_sanitized, text FROM bodies WHERE message_id = $id";
            command.Parameters.AddWithValue("$id", messageId);
            using var reader = command.ExecuteReader();
            ct.ThrowIfCancellationRequested();
            return reader.Read()
                ? new MessageBody(reader.IsDBNull(1) ? "" : reader.GetString(1), reader.IsDBNull(0) ? "" : reader.GetString(0))
                : null;
        }, ct);
    }

    private IReadOnlyList<MailProvider> ListProviders(CancellationToken ct = default)
        => _catalog.Providers;

    private Account AddAccount(AddAccountRequest request, CancellationToken ct = default)
    {
        if (!request.Email.Contains('@', StringComparison.Ordinal))
        {
            throw EngineException.Invalid("email");
        }

        var provider = _catalog.ByEmail(request.Email);
        var filled = ProviderCatalog.WithDefaults(request, provider);
        var account = new Account(
            Guid.NewGuid().ToString("N"),
            filled.Email,
            filled.DisplayName ?? filled.Email.Split('@')[0],
            provider?.Id ?? "custom",
            string.IsNullOrEmpty(filled.ApiToken) ? "standard" : "enhanced");
        WithCredentialRollback([account.Id, $"token:{account.Id}", $"access:{account.Id}", $"refresh:{account.Id}"], () =>
            _store.InsertAccountConfiguration(account, SerializeImap(filled), () => SaveSecrets(account.Id, filled)));

        return account;
    }

    private IReadOnlyList<Account> ListAccounts(CancellationToken ct = default)
        => _store.ListAccounts();

    private void RemoveAccount(string accountId, CancellationToken ct = default)
    {
        // Never contact IMAP/SMTP here: this only disconnects the local account.
        WithCredentialRollback([accountId, $"token:{accountId}", $"access:{accountId}", $"refresh:{accountId}"], () =>
        {
            _store.DeleteAccount(accountId, () =>
            {
                _secrets.Delete(accountId);
                _secrets.Delete($"token:{accountId}");
                _secrets.Delete($"access:{accountId}");
                _secrets.Delete($"refresh:{accountId}");
            });
        });
    }

    private AccountSettings GetAccountSettings(string accountId)
    {
        var account = _store.GetAccount(accountId) ?? throw EngineException.NotFound();
        var configuration = LoadImap(accountId, includeSecrets: false)
            ?? ProviderCatalog.WithDefaults(new AddAccountRequest { Email = account.Email }, _catalog.ByEmail(account.Email));
        return new(accountId, account.Email, account.DisplayName, configuration.Username ?? account.Email,
            configuration.ImapHost ?? "", configuration.ImapPort ?? 993, configuration.ImapStartTls,
            configuration.SmtpHost ?? "", configuration.SmtpPort ?? 465, configuration.SmtpStartTls);
    }

    private Account UpdateAccount(string accountId, UpdateAccountRequest request)
    {
        var account = _store.GetAccount(accountId) ?? throw EngineException.NotFound();
        if (string.IsNullOrWhiteSpace(request.DisplayName) || string.IsNullOrWhiteSpace(request.Username))
            throw EngineException.Invalid("显示名称和登录用户名不能为空");
        static bool ValidHost(string host) => !string.IsNullOrWhiteSpace(host)
            && Uri.CheckHostName(host.Trim()) != UriHostNameType.Unknown;
        if (!ValidHost(request.ImapHost) || !ValidHost(request.SmtpHost) || request.ImapPort == 0 || request.SmtpPort == 0)
            throw EngineException.Invalid("请填写有效的邮件服务器地址和端口");
        var old = LoadImap(accountId, includeSecrets: false) ?? new AddAccountRequest { Email = account.Email };
        // UIDVALIDITY is scoped to a mailbox, not a server. Reusing this account's
        // cached UIDs or queued operations against another identity is unsafe.
        // Endpoint/security changes for the same host remain editable.
        if (!string.Equals(request.ImapHost.Trim().TrimEnd('.'), (old.ImapHost ?? "").Trim().TrimEnd('.'), StringComparison.OrdinalIgnoreCase)
            || !string.Equals(request.Username.Trim(), old.Username ?? account.Email, StringComparison.Ordinal))
            throw EngineException.Invalid("更换 IMAP 服务器或登录用户名，请先添加新账户，确认邮件后再移除旧账户。");
        var changed = old with
        {
            DisplayName = request.DisplayName.Trim(),
            Username = request.Username.Trim(),
            ImapHost = request.ImapHost.Trim(),
            ImapPort = request.ImapPort,
            ImapStartTls = request.ImapStartTls,
            SmtpHost = request.SmtpHost.Trim(),
            SmtpPort = request.SmtpPort,
            SmtpStartTls = request.SmtpStartTls,
        };
        var updated = account with { DisplayName = changed.DisplayName! };
        WithCredentialRollback(string.IsNullOrEmpty(request.Password) ? [] : [accountId], () =>
            _store.UpdateAccountConfiguration(updated, SerializeImap(changed), () =>
            {
                if (!string.IsNullOrEmpty(request.Password)) _secrets.Set(accountId, request.Password);
            }));
        return updated;
    }

    private void WithCredentialRollback(string[] keys, Action mutation)
    {
        var previous = keys.ToDictionary(key => key, key => _secrets.Get(key));
        try { mutation(); }
        catch
        {
            var restoreFailed = false;
            foreach (var (key, value) in previous)
            {
                try
                {
                    if (_secrets.Get(key) == value) continue;
                    if (value is null) _secrets.Delete(key); else _secrets.Set(key, value);
                }
                catch { restoreFailed = true; }
            }
            if (restoreFailed) throw EngineException.Storage("账号未更改，但凭据恢复失败，请重新设置密码");
            throw;
        }
    }

    private ConnectProbe TestAccount(AddAccountRequest request, CancellationToken ct = default)
    {
        if (!request.Email.Contains('@', StringComparison.Ordinal))
        {
            throw EngineException.Invalid("email");
        }

        var provider = _catalog.ByEmail(request.Email);
        var filled = ProviderCatalog.WithDefaults(request, provider);
        var steps = new List<ProbeStep>();
        try
        {
            using var client = ConnectImap(filled, provider, ct);
            steps.Add(new ProbeStep("dns", "ok", filled.ImapHost ?? ""));
            steps.Add(new ProbeStep("tcp", "ok", (filled.ImapPort ?? 993).ToString()));
            steps.Add(new ProbeStep("tls", "ok", "encrypted"));
            steps.Add(new ProbeStep("certificate", "ok", filled.ImapHost ?? ""));
            steps.Add(new ProbeStep("auth", "ok", filled.Email));
            var folders = client.GetFolders(client.PersonalNamespaces[0]);
            steps.Add(new ProbeStep("folders", "ok", folders.Count.ToString()));
            client.Disconnect(true, ct);
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            steps.Add(new ProbeStep("auth", "failed", Map(ex).Message));
        }

        return new ConnectProbe(steps);
    }

    private IReadOnlyList<Folder> ListFolders(string accountId, CancellationToken ct = default)
    {
        lock (_gate)
        {
            var existing = _store.ListFolders(accountId);
            if (existing.Count > 0)
            {
                return existing;
            }
        }

        DiscoverFolders(accountId, ct);
        return _store.ListFolders(accountId);
    }

    private void SyncFolder(string folderId, CancellationToken ct = default)
    {
        FlushOps(ct);
        SyncHeaders(folderId, ct);
        return;
    }

    private void Tick(string? accountId = null, bool idle = false, CancellationToken ct = default)
    {
        _ = idle;
        _ = FlushDueSends(ct);
        FlushOps(ct);
        var accounts = accountId is null
            ? _store.ListAccounts()
            : _store.ListAccounts().Where(a => a.Id == accountId).ToList();
        foreach (var account in accounts)
        {
            var folders = ListFolders(account.Id, ct);
            foreach (var folder in folders)
            {
                SyncHeaders(folder.Id, ct);
            }
        }

        return;
    }

    private IReadOnlyList<MessageRow> ListMessages(string folderId, CancellationToken ct = default)
        => _store.ListMessages(folderId);

    private IReadOnlyList<MessageRow> UnifiedInbox(CancellationToken ct = default)
        => _store.UnifiedInbox();

    private MessageBody GetBody(string messageId, CancellationToken ct = default)
        => FetchBody(messageId, ct);

    private void SetFlags(string messageId, MailFlags flags, CancellationToken ct = default)
    {
        lock (_gate)
        {
            _store.SetMessageFlags(messageId, flags);
            var meta = _store.GetMessageMeta(messageId);
            if (meta is { } m)
            {
                _store.EnqueueOp(m.AccountId, "flag", JsonSerializer.Serialize(new { id = messageId, uid = m.Uid, source = m.FolderPath, uidvalidity = _store.GetFolderUidValidity(m.FolderId), flags = flags.ToBits() }));
            }
        }

        FlushOps(ct);
        return;
    }

    private void DeleteMessage(string messageId, CancellationToken ct = default)
    {
        lock (_gate)
        {
            var meta = _store.GetMessageMeta(messageId);
            _store.DeleteMessage(messageId);
            if (meta is { } m)
            {
                _store.EnqueueOp(m.AccountId, "delete", JsonSerializer.Serialize(new { id = messageId, uid = m.Uid, source = m.FolderPath, uidvalidity = _store.GetFolderUidValidity(m.FolderId) }));
            }
        }

        FlushOps(ct);
        return;
    }

    private string Send(SendRequest request, CancellationToken ct = default)
    {
        var id = Guid.NewGuid().ToString("N");
        var json = JsonSerializer.Serialize(new
        {
            to = request.To,
            cc = request.Cc,
            bcc = request.Bcc,
            subject = request.Subject,
            body_text = request.BodyText,
            body_html = request.BodyHtml,
            in_reply_to = request.InReplyTo,
        });
        lock (_gate)
        {
            if (request.UndoWindowSecs == 0)
            {
                _store.InsertOutbox(id, request.AccountId, json, "queued");
            }
            else
            {
                var sendAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() + request.UndoWindowSecs;
                _store.InsertOutbox(id, request.AccountId, json, "queued", sendAt);
                return id;
            }
        }

        FlushSend(id, ct);
        return id;
    }

    private void UndoSend(string draftId, CancellationToken ct = default)
    {
        lock (_gate)
        {
            var row = _store.GetOutbox(draftId);
            if (row is null)
            {
                throw EngineException.NotFound();
            }

            if (row.Value.State is "queued" or "scheduled" or "draft")
            {
                _store.SetOutboxState(draftId, "cancelled", null);
                return;
            }
        }

        throw EngineException.NotFound();
    }

    private uint FlushDueSends(CancellationToken ct = default)
    {
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        IReadOnlyList<string> ids;
        lock (_gate)
        {
            ids = _store.DueOutbox(now);
        }

        uint n = 0;
        foreach (var id in ids)
        {
            lock (_gate)
            {
                _store.SetOutboxState(id, "queued", null);
            }

            try
            {
                FlushSend(id, ct);
                n++;
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception)
            {
            }
        }

        return n;
    }

    private IReadOnlyList<MessageRow> Search(string query, CancellationToken ct = default)
        => _store.Search(query);

    private IReadOnlyList<AttachmentHandle> ListAttachments(string messageId, CancellationToken ct = default)
    {
        var listed = _store.ListAttachments(messageId);
        if (listed.Count == 0)
        {
            try
            {
                FetchBody(messageId, ct);
            }
            catch (EngineException)
            {
            }

            listed = _store.ListAttachments(messageId);
        }

        return listed;
    }

    private void DiscoverFolders(string accountId, CancellationToken ct)
    {
        var req = LoadImap(accountId);
        if (req is null || string.IsNullOrEmpty(req.ImapHost) || (string.IsNullOrEmpty(req.Password) && string.IsNullOrEmpty(req.AccessToken)))
        {
            return;
        }

        var provider = _catalog.ByEmail(req.Email);
        using var client = ConnectImap(req, provider, ct);
        var personal = client.GetFolder(client.PersonalNamespaces[0]);
        var folders = personal.GetSubfolders(false, ct).ToList();
        try
        {
            folders.Add(client.Inbox);
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception)
        {
        }

        foreach (var remote in folders.DistinctBy(f => f.FullName))
        {
            var flags = remote.Attributes.ToString().Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            var role = FolderRoles.FromFlagsAndName(flags, remote.FullName);
            if (string.Equals(remote.FullName, "INBOX", StringComparison.OrdinalIgnoreCase))
            {
                role = "inbox";
            }

            var folder = new Folder($"{accountId}:{remote.FullName}", remote.FullName, role, 0);
            lock (_gate)
            {
                _store.UpsertFolder(folder, accountId);
            }
        }

        client.Disconnect(true, ct);
    }

    private void SyncHeaders(string folderId, CancellationToken ct)
    {
        var folder = _store.GetFolder(folderId) ?? throw EngineException.NotFound();
        var accountId = _store.FolderAccountId(folderId) ?? throw EngineException.NotFound();
        var req = LoadImap(accountId) ?? throw EngineException.Invalid("imap");
        var provider = _catalog.ByEmail(req.Email);
        using var client = ConnectImap(req, provider, ct);
        var remote = client.GetFolder(folder.Path);
        remote.Open(FolderAccess.ReadOnly, ct);
        _store.SetFolderUidValidity(folderId, remote.UidValidity);
        var cursor = _store.FolderCursor(folderId);
        var fromUid = _store.NeedsEnvelopeMetadata(folderId) ? 1u : cursor;
        var range = new UniqueIdRange(new UniqueId(Math.Max(fromUid, 1u)), UniqueId.MaxValue);
        IList<IMessageSummary> items;
        try
        {
            items = remote.Fetch(range, MessageSummaryItems.UniqueId | MessageSummaryItems.Envelope | MessageSummaryItems.Flags | MessageSummaryItems.InternalDate | MessageSummaryItems.BodyStructure, ct);
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception)
        {
            items = remote.Fetch(0, -1, MessageSummaryItems.UniqueId | MessageSummaryItems.Envelope | MessageSummaryItems.Flags | MessageSummaryItems.InternalDate | MessageSummaryItems.BodyStructure, ct);
        }

        uint lastUid = cursor;
        foreach (var item in items)
        {
            if (item.UniqueId.Id < fromUid)
            {
                continue;
            }

            var subject = item.Envelope?.Subject ?? "";
            var from = item.Envelope?.From?.Mailboxes.Select(m => new { name = m.Name, email = m.Address }).ToList() ?? [];
            var flags = new MailFlags(
                Seen: item.Flags?.HasFlag(MessageFlags.Seen) == true,
                Flagged: item.Flags?.HasFlag(MessageFlags.Flagged) == true,
                Answered: item.Flags?.HasFlag(MessageFlags.Answered) == true,
                Draft: item.Flags?.HasFlag(MessageFlags.Draft) == true,
                Deleted: item.Flags?.HasFlag(MessageFlags.Deleted) == true);
            var snippet = subject.Length > 140 ? subject[..140] : subject;
            var hasAtt = item.BodyParts?.Any(p => p.ContentDisposition?.IsAttachment == true) == true;
            lock (_gate)
            {
                _store.UpsertMessage(
                    accountId,
                    folderId,
                    item.UniqueId.Id,
                    subject,
                    JsonSerializer.Serialize(from),
                    item.InternalDate?.ToUnixTimeSeconds() ?? 0,
                    snippet,
                    flags,
                    hasAtt,
                    item.Envelope?.MessageId,
                    item.Envelope?.MessageId,
                    JsonSerializer.Serialize(item.Envelope?.To?.Mailboxes.Select(address => address.Address).ToArray() ?? []),
                    JsonSerializer.Serialize(item.Envelope?.Cc?.Mailboxes.Select(address => address.Address).ToArray() ?? []),
                    JsonSerializer.Serialize(item.Envelope?.ReplyTo?.Mailboxes.Select(address => address.Address).ToArray() ?? []));
            }

            lastUid = Math.Max(lastUid, item.UniqueId.Id + 1);
        }

        // A successful FETCH is not by itself evidence that omitted UIDs vanished.
        // Compare SEARCH snapshots around the lightweight flags fetch; only a complete,
        // stable response can remove local rows. No message bodies are downloaded.
        var expectedUids = remote.Search(global::MailKit.Search.SearchQuery.All, ct);
        var snapshot = new Dictionary<uint, MailFlags>();
        if (expectedUids.Count > 0)
        {
            var summaries = remote.Fetch(expectedUids, MessageSummaryItems.UniqueId | MessageSummaryItems.Flags, ct);
            foreach (var summary in summaries)
            {
                if (!summary.UniqueId.IsValid || summary.Flags is not { } remoteFlags) continue;
                snapshot[summary.UniqueId.Id] = ToMailFlags(remoteFlags);
            }
        }
        var confirmedUids = remote.Search(global::MailKit.Search.SearchQuery.All, ct);
        var expected = expectedUids.Select(uid => uid.Id).ToHashSet();
        if (!expected.SetEquals(confirmedUids.Select(uid => uid.Id)) || !expected.SetEquals(snapshot.Keys))
            throw EngineException.Invalid("同步期间邮件列表发生变化，请稍后重试");
        _store.ReconcileFolderSnapshot(folderId, remote.UidValidity, snapshot);
        var (total, unread) = _store.MessageCount(folderId);
        _store.SetFolderCursor(folderId, lastUid, total, unread);
        remote.Close(false, ct);
        client.Disconnect(true, ct);
    }

    private static MailFlags ToMailFlags(MessageFlags flags) => new(
        Seen: flags.HasFlag(MessageFlags.Seen), Flagged: flags.HasFlag(MessageFlags.Flagged),
        Answered: flags.HasFlag(MessageFlags.Answered), Draft: flags.HasFlag(MessageFlags.Draft),
        Deleted: flags.HasFlag(MessageFlags.Deleted));

    private MessageBody FetchBody(string messageId, CancellationToken ct)
    {
        var cached = _store.GetBody(messageId);
        if (cached is not null)
        {
            return cached;
        }

        var meta = _store.GetMessageMeta(messageId) ?? throw EngineException.NotFound();
        var req = LoadImap(meta.AccountId) ?? throw EngineException.Invalid("imap");
        var provider = _catalog.ByEmail(req.Email);
        using var client = ConnectImap(req, provider, ct);
        var remote = client.GetFolder(meta.FolderPath);
        remote.Open(FolderAccess.ReadOnly, ct);
        var expectedValidity = _store.GetFolderUidValidity(meta.FolderId);
        if (expectedValidity is null or 0 || remote.UidValidity != expectedValidity)
            throw EngineException.Invalid("邮箱身份已改变，请同步邮件后重新打开");
        var mime = remote.GetMessage(new UniqueId(meta.Uid), ct);
        remote.Close(false, ct);
        client.Disconnect(true, ct);

        var text = mime.TextBody ?? "";
        var html = mime.HtmlBody;
        SanitizeResult sanitized;
        if (!string.IsNullOrEmpty(html))
        {
            sanitized = HtmlSanitizer.Sanitize(html);
        }
        else
        {
            sanitized = new SanitizeResult($"<pre>{HtmlSanitizer.Escape(text)}</pre>", 0);
        }

        if (string.IsNullOrEmpty(text) && !string.IsNullOrEmpty(html))
        {
            text = System.Text.RegularExpressions.Regex.Replace(html, "<[^>]+>", " ");
        }

        lock (_gate)
        {
            _store.PutBody(messageId, sanitized.Html, text);
            var i = 0;
            foreach (var att in mime.Attachments)
            {
                if (att is not MimePart part || part.Content is null)
                {
                    continue;
                }

                using var ms = new MemoryStream();
                part.Content.DecodeTo(ms);
                var bytes = ms.ToArray();
                var handle = new AttachmentHandle(
                    $"{messageId}:{i}",
                    part.FileName ?? "attachment",
                    part.ContentType.MimeType,
                    (ulong)bytes.Length,
                    part.ContentId);
                _store.UpsertAttachment(messageId, handle, bytes);
                i++;
            }
        }

        return new MessageBody(text, sanitized.Html, sanitized.RemoteBlocked);
    }

    private void FlushOps(CancellationToken ct)
    {
        foreach (var (opId, accountId, type, payload) in _store.ListPendingOps())
        {
            ct.ThrowIfCancellationRequested();
            try
            {
                using var doc = JsonDocument.Parse(payload);
                var root = doc.RootElement;
                if (!root.TryGetProperty("source", out var source) || source.ValueKind != JsonValueKind.String || string.IsNullOrEmpty(source.GetString()) ||
                    !root.TryGetProperty("uidvalidity", out var validity) || !validity.TryGetUInt32(out var expectedValidity) || expectedValidity == 0 ||
                    !root.TryGetProperty("uid", out var uidValue) || !uidValue.TryGetUInt32(out var uid) || uid == 0 || type is not ("flag" or "mark_read" or "delete"))
                {
                    _store.MarkOp(opId, "failed");
                    continue;
                }
                var req = LoadImap(accountId);
                if (req is null || string.IsNullOrEmpty(req.ImapHost))
                {
                    _store.MarkOp(opId, "failed");
                    continue;
                }
                using var client = ConnectImap(req, _catalog.ByEmail(req.Email), ct);
                var folder = client.GetFolder(source.GetString()!, ct);
                folder.Open(FolderAccess.ReadWrite, ct);
                if (folder.UidValidity != expectedValidity)
                    throw EngineException.Invalid("source UIDVALIDITY changed");
                var unique = new UniqueId(uid);
                if (type == "mark_read")
                {
                    // Opening a message adds Seen; it must not overwrite flags
                    // another client changed since our cached headers were fetched.
                    folder.AddFlags(unique, MessageFlags.Seen, true, ct);
                }
                else if (type == "flag")
                {
                    var flags = MailFlags.FromBits(root.GetProperty("flags").GetInt64());
                    var set = MessageFlags.None;
                    if (flags.Seen) set |= MessageFlags.Seen;
                    if (flags.Flagged) set |= MessageFlags.Flagged;
                    if (flags.Answered) set |= MessageFlags.Answered;
                    if (flags.Draft) set |= MessageFlags.Draft;
                    if (flags.Deleted) set |= MessageFlags.Deleted;
                    folder.SetFlags(unique, set, true, ct);
                }
                else
                {
                    if (!client.Capabilities.HasFlag(ImapCapabilities.UidPlus))
                        throw EngineException.Invalid("safe delete requires UIDPLUS");
                    folder.AddFlags(unique, MessageFlags.Deleted, true, ct);
                    folder.Expunge(new[] { unique }, ct);
                }
                folder.Close(false, ct);
                client.Disconnect(true, ct);
                _store.MarkOp(opId, "done");
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested) { throw; }
            catch (Exception) { _store.MarkOp(opId, "failed"); }
        }
    }

    private void FlushSend(string draftId, CancellationToken ct)
    {
        try { DeliverSend(draftId, ct); }
        catch
        {
            // Failed attempts must leave the active queue. Never turn an
            // acknowledged SMTP delivery into a retryable message because
            // disconnecting or appending to Sent failed afterwards.
            if (_store.GetOutbox(draftId) is { State: "sending" })
                _store.SetOutboxState(draftId, "failed", "发送未完成，请核对收件人是否收到后再重试");
            throw;
        }
    }

    private void DeliverSend(string draftId, CancellationToken ct)
    {
        var row = _store.GetOutbox(draftId) ?? throw EngineException.NotFound();
        if (row.State == "cancelled")
        {
            return;
        }

        _store.SetOutboxState(draftId, "sending", null);
        using var doc = JsonDocument.Parse(row.Json);
        var to = Strings(doc.RootElement, "to");
        var cc = Strings(doc.RootElement, "cc");
        var bcc = Strings(doc.RootElement, "bcc");
        var subject = doc.RootElement.TryGetProperty("subject", out var s) ? s.GetString() ?? "" : "";
        var bodyText = doc.RootElement.TryGetProperty("body_text", out var b) ? b.GetString() ?? "" : "";
        var bodyHtml = doc.RootElement.TryGetProperty("body_html", out var h) && h.ValueKind == JsonValueKind.String
            ? h.GetString() : null;
        var sig = _store.DefaultSignature(row.AccountId);
        if (!string.IsNullOrEmpty(sig) && !bodyText.Contains(sig, StringComparison.Ordinal))
        {
            bodyText = string.IsNullOrEmpty(bodyText) ? sig : $"{bodyText}\n\n{sig}";
            if (!string.IsNullOrWhiteSpace(bodyHtml))
            {
                // Signatures are stored as plain text. Never interpret their contents
                // as tags, attributes, links, or a second Markdown document.
                var signatureHtml = "<p>" + System.Net.WebUtility.HtmlEncode(sig)
                    .Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n')
                    .Replace("\n", "<br />", StringComparison.Ordinal) + "</p>";
                var closingBody = bodyHtml.LastIndexOf("</body", StringComparison.OrdinalIgnoreCase);
                bodyHtml = closingBody >= 0 ? bodyHtml.Insert(closingBody, signatureHtml) : bodyHtml + signatureHtml;
            }
        }

        var req = LoadImap(row.AccountId) ?? throw EngineException.Invalid("smtp");
        var provider = _catalog.ByEmail(req.Email);
        var message = new MimeMessage();
        message.From.Add(new MailboxAddress(_store.GetAccount(row.AccountId)?.DisplayName ?? "", req.Email));
        foreach (var addr in to) message.To.Add(MailboxAddress.Parse(addr));
        foreach (var addr in cc) message.Cc.Add(MailboxAddress.Parse(addr));
        foreach (var addr in bcc) message.Bcc.Add(MailboxAddress.Parse(addr));
        if (doc.RootElement.TryGetProperty("in_reply_to", out var reply) && reply.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(reply.GetString()))
        {
            message.InReplyTo = reply.GetString();
            message.References.Add(reply.GetString()!);
        }
        message.Subject = subject;
        message.Body = string.IsNullOrWhiteSpace(bodyHtml)
            ? new TextPart("plain") { Text = bodyText }
            : new BodyBuilder { TextBody = bodyText, HtmlBody = bodyHtml }.ToMessageBody();

        using var smtp = new SmtpClient();
        ConfigureCerts(smtp, req.AcceptInvalidCerts);
        var host = req.SmtpHost ?? provider?.SmtpHost ?? throw EngineException.Invalid("smtp host");
        var startTls = req.SmtpStartTls;
        var port = req.SmtpPort ?? provider?.SmtpPort ?? (ushort)(startTls ? 587 : 465);
        smtp.Connect(host, port, startTls ? SecureSocketOptions.StartTls : SecureSocketOptions.SslOnConnect, ct);
        if (!smtp.IsSecure)
        {
            throw EngineException.Tls();
        }

        AuthenticateMail(smtp, req, ct);
        smtp.Send(message, ct);
        _store.SetOutboxState(draftId, "sent", null);
        smtp.Disconnect(true, ct);
        AppendSent(row.AccountId, req, provider, message, ct);
    }

    private void AppendSent(string accountId, AddAccountRequest req, MailProvider? provider, MimeMessage message, CancellationToken ct)
    {
        var sent = _store.FolderByRole(accountId, "sent");
        if (sent is null)
        {
            return;
        }

        try
        {
            using var client = ConnectImap(req, provider, ct);
            var folder = client.GetFolder(sent.Path);
            folder.Open(FolderAccess.ReadWrite, ct);
            folder.Append(message, MessageFlags.Seen, ct);
            folder.Close(false, ct);
            client.Disconnect(true, ct);
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception)
        {
        }
    }

    private ImapClient ConnectImap(AddAccountRequest req, MailProvider? provider, CancellationToken ct)
    {
        var host = req.ImapHost ?? provider?.ImapHost ?? throw EngineException.Invalid("imap host");
        var startTls = req.ImapStartTls;
        var port = req.ImapPort ?? provider?.ImapPort ?? (ushort)(startTls ? 143 : 993);
        var client = new ImapClient();
        try
        {
            ConfigureCerts(client, req.AcceptInvalidCerts);
            client.Timeout = 8000;
            client.Connect(host, port, startTls ? SecureSocketOptions.StartTls : SecureSocketOptions.SslOnConnect, ct);
            if (!client.IsSecure) throw EngineException.Tls();
            var forceId = provider?.Quirks.Contains("imap-id") == true;
            if (forceId || client.Capabilities.HasFlag(ImapCapabilities.Id))
            {
                try
                {
                    client.Identify(new ImapImplementation { Name = "imyemail-cloud", Version = "0.2.2", Vendor = "imy.email" }, ct);
                }
                catch (Exception) when (!forceId && !ct.IsCancellationRequested) { }
            }
            ct.ThrowIfCancellationRequested();
            AuthenticateMail(client, req, ct);
            return client;
        }
        catch
        {
            client.Dispose();
            ct.ThrowIfCancellationRequested();
            throw;
        }
    }

    private static void AuthenticateMail(MailService client, AddAccountRequest req, CancellationToken ct)
    {
        var user = req.Username ?? req.Email;
        if (!string.IsNullOrEmpty(req.AccessToken))
        {
            var oauth = new SaslMechanismOAuth2(user, req.AccessToken);
            client.Authenticate(oauth, ct);
            return;
        }

        client.Authenticate(user, req.Password ?? throw EngineException.AuthFailed(), ct);
    }

    private static void ConfigureCerts(MailService client, bool acceptInvalid)
    {
        if (!acceptInvalid)
        {
            return;
        }

        client.ServerCertificateValidationCallback = (_, _, _, errors) =>
            errors is SslPolicyErrors.None or SslPolicyErrors.RemoteCertificateNameMismatch or SslPolicyErrors.RemoteCertificateChainErrors || true;
    }

    private static string SerializeImap(AddAccountRequest req)
        => JsonSerializer.Serialize(new
        {
            email = req.Email,
            imap_host = req.ImapHost,
            imap_port = req.ImapPort,
            imap_starttls = req.ImapStartTls,
            smtp_host = req.SmtpHost,
            smtp_port = req.SmtpPort,
            smtp_starttls = req.SmtpStartTls,
            accept_invalid_certs = req.AcceptInvalidCerts,
            username = req.Username,
            api_base = req.ApiBase,
        });
    private void SaveSecrets(string id, AddAccountRequest req)
    {
        if (!string.IsNullOrEmpty(req.Password))
        {
            _secrets.Set(id, req.Password);
        }

        if (!string.IsNullOrEmpty(req.ApiToken))
        {
            _secrets.Set($"token:{id}", req.ApiToken);
        }

        if (!string.IsNullOrEmpty(req.AccessToken))
        {
            _secrets.Set($"access:{id}", req.AccessToken);
        }

        if (!string.IsNullOrEmpty(req.RefreshToken))
        {
            _secrets.Set($"refresh:{id}", req.RefreshToken);
        }
    }

    private AddAccountRequest? LoadImap(string id, bool includeSecrets = true)
    {
        var raw = _store.GetKv($"imap:{id}");
        if (raw is null)
        {
            return null;
        }

        using var doc = JsonDocument.Parse(raw);
        var root = doc.RootElement;
        string? Str(string name) => root.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;
        ushort? Port(string name) => root.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number && v.TryGetUInt16(out var n) ? n : null;
        bool Flag(string name) => root.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.True;

        var email = Str("email") ?? "";
        var req = new AddAccountRequest
        {
            Email = email,
            ImapHost = Str("imap_host"),
            ImapPort = Port("imap_port"),
            ImapStartTls = Flag("imap_starttls"),
            SmtpHost = Str("smtp_host"),
            SmtpPort = Port("smtp_port"),
            SmtpStartTls = Flag("smtp_starttls"),
            AcceptInvalidCerts = Flag("accept_invalid_certs"),
            Username = Str("username"),
            ApiBase = Str("api_base"),
            Password = includeSecrets ? _secrets.Get(id) : null,
            ApiToken = includeSecrets ? _secrets.Get($"token:{id}") : null,
            AccessToken = includeSecrets ? _secrets.Get($"access:{id}") : null,
            RefreshToken = includeSecrets ? _secrets.Get($"refresh:{id}") : null,
        };
        // Saved TLS choices must remain editable even for known provider domains.
        var filled = ProviderCatalog.WithDefaults(req, _catalog.ByEmail(email));
        return filled with
        {
            ImapStartTls = root.TryGetProperty("imap_starttls", out _) ? req.ImapStartTls : filled.ImapStartTls,
            SmtpStartTls = root.TryGetProperty("smtp_starttls", out _) ? req.SmtpStartTls : filled.SmtpStartTls,
        };
    }

    private static List<string> Strings(JsonElement root, string name)
    {
        if (!root.TryGetProperty(name, out var v) || v.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        return v.EnumerateArray().Select(x => x.GetString() ?? "").Where(s => s.Length > 0).ToList();
    }

    private static EngineException Map(Exception ex) => ex switch
    {
        EngineException e => e,
        SslHandshakeException => EngineException.Tls(),
        TimeoutException => EngineException.Network("timeout"),
        _ => EngineException.Network(ex.GetType().Name),
    };
}
