using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Chck.Mail.Tests;

public sealed class AccountManagementTests
{
    private static string DatabasePath() => Path.Combine(Path.GetTempPath(), "chck-accounts-" + Guid.NewGuid().ToString("N"), "mail.db");

    private static AddAccountRequest NewAccount(string email = "first@qq.com") => new()
    {
        Email = email,
        DisplayName = "Original",
        Password = "original-password",
        ApiToken = "api-token",
        AccessToken = "access-token",
        RefreshToken = "refresh-token",
    };

    private static UpdateAccountRequest Edit(AccountSettings settings, string? password = null) => new()
    {
        DisplayName = "Updated",
        Username = settings.Username,
        ImapHost = settings.ImapHost,
        ImapPort = 1993,
        ImapStartTls = true,
        SmtpHost = "smtp.example.test",
        SmtpPort = 1465,
        SmtpStartTls = false,
        Password = password,
    };

    [Fact]
    public async Task FailedAccountCreationDoesNotLeaveAccountConfigurationOrSecret()
    {
        var path = DatabasePath();
        var secrets = new FaultingSecrets { FailNextWriteAfterMutation = true };
        using var engine = new MailKitEngine(path, secrets);
        await Assert.ThrowsAsync<IOException>(() => engine.AddAccountAsync(NewAccount()));
        Assert.Empty(await engine.ListAccountsAsync());
        Assert.Empty(secrets.Snapshot());
        using var connection = new SqliteConnection($"Data Source={path}");
        connection.Open();
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT COUNT(*) FROM settings WHERE key LIKE 'imap:%'";
        Assert.Equal(0L, (long)command.ExecuteScalar()!);
    }

    [Fact]
    public async Task SettingsOmitSecretsAndEditsPreserveAccountIdentityAndOtherAccount()
    {
        var path = DatabasePath();
        var secrets = new FaultingSecrets();
        using var engine = new MailKitEngine(path, secrets);
        var first = await engine.AddAccountAsync(NewAccount());
        var second = await engine.AddAccountAsync(NewAccount("second@qq.com"));
        secrets.ThrowOnRead = true;
        var settings = await engine.GetAccountSettingsAsync(first.Id);
        Assert.Equal("imap.qq.com", settings.ImapHost);
        Assert.Equal(first.Email, settings.Username);
        Assert.DoesNotContain("password", System.Text.Json.JsonSerializer.Serialize(settings), StringComparison.OrdinalIgnoreCase);
        // Display-only edits must not even read a credential.
        await engine.UpdateAccountAsync(first.Id, Edit(settings, ""));
        secrets.ThrowOnRead = false;
        var changed = await engine.GetAccountSettingsAsync(first.Id);
        Assert.Equal(first.Email, changed.Email);
        Assert.Equal("Updated", changed.DisplayName);
        Assert.Equal(first.Email, changed.Username);
        Assert.Equal((ushort)1993, changed.ImapPort);
        Assert.Equal((ushort)1465, changed.SmtpPort);
        Assert.True(changed.ImapStartTls);
        Assert.False(changed.SmtpStartTls);
        Assert.Equal("original-password", secrets.Get(first.Id));
        Assert.Equal("Original", (await engine.GetAccountSettingsAsync(second.Id)).DisplayName);
        Assert.Equal("api-token", secrets.Get($"token:{first.Id}"));
        Assert.Equal("access-token", secrets.Get($"access:{first.Id}"));
        Assert.Equal("refresh-token", secrets.Get($"refresh:{first.Id}"));
    }

    [Fact]
    public async Task ImapIdentityChangesAreRejectedWithoutTouchingCachedMailQueueOrCredentials()
    {
        var path = DatabasePath();
        var secrets = SecretStore.Memory();
        using var engine = new MailKitEngine(path, secrets);
        var account = await engine.AddAccountAsync(NewAccount());
        Seed(path, account.Id);
        var before = await engine.GetAccountSettingsAsync(account.Id);
        foreach (var request in new[]
        {
            Edit(before, "new-password") with { ImapHost = "another.example.test" },
            Edit(before, "new-password") with { Username = "another-user" },
        })
        {
            var error = await Assert.ThrowsAsync<EngineException>(() => engine.UpdateAccountAsync(account.Id, request));
            Assert.Contains("添加新账户", error.Message, StringComparison.Ordinal);
        }
        Assert.Equal(before, await engine.GetAccountSettingsAsync(account.Id));
        Assert.Equal("original-password", secrets.Get(account.Id));
        using var store = new MailStore(path);
        Assert.Single(store.Search("searchable"));
        Assert.Single(store.ListPendingOps());
        Assert.Single(store.DueOutbox(long.MaxValue));
        Assert.NotNull(store.GetBody(Assert.Single(store.ListMessages(account.Id + ":INBOX")).Id));
    }

    [Fact]
    public async Task SmtpAndTransportChangesKeepExistingCachedMailboxAndIdentity()
    {
        var path = DatabasePath();
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        var account = await engine.AddAccountAsync(NewAccount());
        Seed(path, account.Id);
        var before = await engine.GetAccountSettingsAsync(account.Id);
        await engine.UpdateAccountAsync(account.Id, Edit(before) with { ImapHost = before.ImapHost.ToUpperInvariant() + "." });
        var after = await engine.GetAccountSettingsAsync(account.Id);
        Assert.Equal(before.Username, after.Username);
        Assert.Equal((ushort)1993, after.ImapPort);
        Assert.True(after.ImapStartTls);
        Assert.Equal("smtp.example.test", after.SmtpHost);
        Assert.Equal((ushort)1465, after.SmtpPort);
        Assert.False(after.SmtpStartTls);
        using var store = new MailStore(path);
        Assert.Single(store.Search("searchable"));
        Assert.Single(store.ListPendingOps());
        Assert.Single(store.DueOutbox(long.MaxValue));
    }

    [Fact]
    public async Task PasswordChangeIsStoredOnlyInCredentialStore()
    {
        var path = DatabasePath();
        var secrets = SecretStore.Memory();
        using var engine = new MailKitEngine(path, secrets);
        var account = await engine.AddAccountAsync(NewAccount());
        await engine.UpdateAccountAsync(account.Id, Edit(await engine.GetAccountSettingsAsync(account.Id), "replacement-secret"));
        Assert.Equal("replacement-secret", secrets.Get(account.Id));
        using var store = new MailStore(path);
        Assert.DoesNotContain("replacement-secret", store.GetKv($"imap:{account.Id}")!, StringComparison.Ordinal);
    }

    [Fact]
    public async Task CredentialWriteFailureRollsBackSettingsAndName()
    {
        var path = DatabasePath();
        var secrets = new FaultingSecrets();
        using var engine = new MailKitEngine(path, secrets);
        var account = await engine.AddAccountAsync(NewAccount());
        var before = await engine.GetAccountSettingsAsync(account.Id);
        secrets.FailNextWriteAfterMutation = true;
        await Assert.ThrowsAsync<IOException>(() => engine.UpdateAccountAsync(account.Id, Edit(before, "new-password")));
        Assert.Equal(before, await engine.GetAccountSettingsAsync(account.Id));
        Assert.Equal("original-password", secrets.Get(account.Id));
    }

    [Fact]
    public async Task InvalidServerAndMissingAccountCannotMutateCredentials()
    {
        var secrets = SecretStore.Memory();
        using var engine = new MailKitEngine(DatabasePath(), secrets);
        var account = await engine.AddAccountAsync(NewAccount());
        var settings = await engine.GetAccountSettingsAsync(account.Id);
        await Assert.ThrowsAsync<EngineException>(() => engine.UpdateAccountAsync(account.Id, Edit(settings, "new") with { ImapHost = "https://imap.example.test" }));
        await Assert.ThrowsAsync<EngineException>(() => engine.UpdateAccountAsync(account.Id, Edit(settings, "new") with { SmtpPort = 0 }));
        await Assert.ThrowsAsync<EngineException>(() => engine.UpdateAccountAsync("missing", Edit(settings, "new")));
        Assert.Equal(settings, await engine.GetAccountSettingsAsync(account.Id));
        Assert.Equal("original-password", secrets.Get(account.Id));
    }

    [Fact]
    public async Task RemovalClearsOnlySelectedLocalAccountIncludingFtsBodiesAttachmentsAndQueues()
    {
        var path = DatabasePath();
        var secrets = SecretStore.Memory();
        using var engine = new MailKitEngine(path, secrets);
        var first = await engine.AddAccountAsync(NewAccount());
        var second = await engine.AddAccountAsync(NewAccount("second@qq.com"));
        Seed(path, first.Id);
        Seed(path, second.Id);
        await engine.RemoveAccountAsync(first.Id);
        Assert.Equal(second.Id, Assert.Single(await engine.ListAccountsAsync()).Id);
        using var store = new MailStore(path);
        Assert.Null(store.GetKv($"imap:{first.Id}"));
        Assert.NotNull(store.GetKv($"imap:{second.Id}"));
        Assert.Empty(store.ListFolders(first.Id));
        Assert.Single(store.ListFolders(second.Id));
        Assert.Equal(second.Id + "-draft", Assert.Single(store.DueOutbox(long.MaxValue)));
        using var connection = new SqliteConnection($"Data Source={path}");
        connection.Open();
        foreach (var table in new[] { "messages", "bodies", "attachments", "search_index", "ops_queue" })
        {
            using var command = connection.CreateCommand();
            command.CommandText = $"SELECT COUNT(*) FROM {table}";
            Assert.Equal(1L, (long)command.ExecuteScalar()!);
        }
        foreach (var prefix in new[] { "", "token:", "access:", "refresh:" })
        {
            Assert.Null(secrets.Get(prefix + first.Id));
            Assert.NotNull(secrets.Get(prefix + second.Id));
        }
    }

    [Fact]
    public async Task CredentialDeletionFailureRollsBackLocalRemovalAndPreviouslyDeletedSecrets()
    {
        var path = DatabasePath();
        var secrets = new FaultingSecrets();
        using var engine = new MailKitEngine(path, secrets);
        var account = await engine.AddAccountAsync(NewAccount());
        Seed(path, account.Id);
        secrets.FailDeleteKey = $"access:{account.Id}";
        await Assert.ThrowsAsync<IOException>(() => engine.RemoveAccountAsync(account.Id));
        Assert.Single(await engine.ListAccountsAsync());
        using var store = new MailStore(path);
        Assert.Single(store.Search("searchable"));
        Assert.Single(store.DueOutbox(long.MaxValue));
        Assert.NotNull(store.GetKv($"imap:{account.Id}"));
        Assert.Equal("original-password", secrets.Get(account.Id));
        Assert.Equal("api-token", secrets.Get($"token:{account.Id}"));
        Assert.Equal("access-token", secrets.Get($"access:{account.Id}"));
        Assert.Equal("refresh-token", secrets.Get($"refresh:{account.Id}"));
    }

    [Fact]
    public void OverlayPropagatesVaultDeleteFailureAndKeepsFallback()
    {
        var path = DatabasePath();
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var primary = new FaultingSecrets();
        var fallback = SecretStore.OpenBeside(path);
        var overlay = new OverlaySecretStore(primary, fallback);
        primary.Set("account", "password");
        fallback.Set("account", "legacy-password");
        primary.FailDeleteKey = "account";
        Assert.Throws<IOException>(() => overlay.Delete("account"));
        Assert.Equal("legacy-password", fallback.Get("account"));
    }

    private static void Seed(string path, string accountId)
    {
        using var store = new MailStore(path);
        var folder = accountId + ":INBOX";
        store.UpsertFolder(new(folder, "INBOX", "inbox", 1), accountId);
        store.UpsertMessage(accountId, folder, 1, "searchable mail", "[]", 1, "body", new(), true, null, null);
        var message = Assert.Single(store.ListMessages(folder));
        store.PutBody(message.Id, "<p>body</p>", "body");
        store.UpsertAttachment(message.Id, new(accountId + "-attachment", "test.txt", "text/plain", 1, null), [1]);
        store.InsertOutbox(accountId + "-draft", accountId, "{}", "queued", 1);
        store.EnqueueOp(accountId, "mark_read", "{}");
    }

    private sealed class FaultingSecrets : ISecretStore
    {
        private readonly ISecretStore _inner = SecretStore.Memory();
        public bool ThrowOnRead { get; set; }
        public bool FailNextWriteAfterMutation { get; set; }
        public string? FailDeleteKey { get; set; }
        public string? Get(string id) => ThrowOnRead ? throw new InvalidOperationException("No secret reads allowed") : _inner.Get(id);
        public void Set(string id, string secret)
        {
            _inner.Set(id, secret);
            if (!FailNextWriteAfterMutation) return;
            FailNextWriteAfterMutation = false;
            throw new IOException("Test vault write failure");
        }
        public void Delete(string id)
        {
            if (id == FailDeleteKey) throw new IOException("Test vault delete failure");
            _inner.Delete(id);
        }
        public IReadOnlyDictionary<string, string> Snapshot() => _inner.Snapshot();
    }
}
