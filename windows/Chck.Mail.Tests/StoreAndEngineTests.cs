using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Xunit;

namespace Chck.Mail.Tests;

public class StoreAndEngineTests
{
    [Fact]
    public void SchemaMigratesToV3()
    {
        using var store = MailStore.OpenInMemory();
        Assert.Equal(MailStore.SchemaVersion, store.UserVersion());
    }

    [Fact]
    public void InsertAccountAndFolderRoundTrip()
    {
        using var store = MailStore.OpenInMemory();
        var acc = new Account("a1", "dev@qq.com", "Dev", "qq");
        store.InsertAccount(acc);
        store.UpsertFolder(new Folder("a1:INBOX", "INBOX", "inbox", 1), acc.Id);
        store.UpsertMessage(acc.Id, "a1:INBOX", 1, "Hello from testkit", """[{"email":"alice@imyemail.test"}]""", 0, "Hello", new MailFlags(), false, "<id@imyemail.test>", null);
        var rows = store.ListMessages("a1:INBOX");
        Assert.Single(rows);
        Assert.Equal("alice@imyemail.test", rows[0].From);
        Assert.True(rows[0].Unread);
        var hits = store.Search("Hello");
        Assert.Single(hits);
        store.InsertOutbox("d1", acc.Id, "{}", "queued", 1);
        Assert.Single(store.DueOutbox(2));
        Assert.Empty(store.DueOutbox(0));
    }

    [Fact]
    public void SecretsStayBesideDbNotInSqlite()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        var db = Path.Combine(dir, "mail.db");
        var secrets = SecretStore.OpenBeside(db);
        secrets.Set("acc1", "auth-code");
        Assert.Equal("auth-code", secrets.Get("acc1"));
        Assert.True(File.Exists(db + ".secrets"));
        var json = File.ReadAllText(db + ".secrets");
        Assert.Contains("auth-code", json, StringComparison.Ordinal);
        using var store = new MailStore(db);
        store.InsertAccount(new Account("acc1", "dev@qq.com", "Dev", "qq"));
        store.SetKv("imap:acc1", """{"email":"dev@qq.com"}""");
        AssertSecretsAbsentFromDatabase(db, "auth-code");
    }

    [Fact]
    public void OverlayMigratesSidecarThenReadsPrimary()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-ov-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        var db = Path.Combine(dir, "mail.db");
        var sidecar = SecretStore.OpenBeside(db);
        sidecar.Set("a1", "from-file");
        var overlay = new OverlaySecretStore(SecretStore.Memory(), sidecar);
        Assert.Equal("from-file", overlay.Get("a1"));
        var leftover = File.Exists(db + ".secrets") ? File.ReadAllText(db + ".secrets") : "";
        Assert.DoesNotContain("from-file", leftover, StringComparison.Ordinal);
        overlay.Set("a1", "from-os");
        Assert.Equal("from-os", overlay.Get("a1"));
        leftover = File.Exists(db + ".secrets") ? File.ReadAllText(db + ".secrets") : "";
        Assert.DoesNotContain("from-os", leftover, StringComparison.Ordinal);
    }

    [Fact]
    public async Task MailKitEngineOsSecretsSkipSidecar()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-os-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        var db = Path.Combine(dir, "mail.db");
        var os = SecretStore.Memory();
        using var engine = new MailKitEngine(db, new OverlaySecretStore(os, SecretStore.OpenBeside(db)));
        var acc = await engine.AddAccountAsync(new AddAccountRequest { Email = "a@gmail.com", Password = "os-pass" });
        var leftover = File.Exists(db + ".secrets") ? File.ReadAllText(db + ".secrets") : "";
        Assert.DoesNotContain("os-pass", leftover, StringComparison.Ordinal);
        Assert.Equal("os-pass", os.Get(acc.Id));
        await engine.RemoveAccountAsync(acc.Id);
        Assert.Null(os.Get(acc.Id));
    }

    [Fact]
    public async Task MailKitEngineOauthTokensStayBesideDb()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-oauth-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        var db = Path.Combine(dir, "mail.db");
        using var engine = new MailKitEngine(db, SecretStore.OpenBeside(db));
        var acc = await engine.AddAccountAsync(new AddAccountRequest
        {
            Email = "a@gmail.com",
            AccessToken = "ya29.win",
            RefreshToken = "1//win",
        });
        AssertSecretsAbsentFromDatabase(db, "ya29.win", "1//win");
        var sidecar = File.ReadAllText(db + ".secrets");
        Assert.Contains("ya29.win", sidecar, StringComparison.Ordinal);
        Assert.Contains("1//win", sidecar, StringComparison.Ordinal);
        await engine.RemoveAccountAsync(acc.Id);
        sidecar = File.Exists(db + ".secrets") ? File.ReadAllText(db + ".secrets") : "";
        Assert.DoesNotContain("ya29.win", sidecar, StringComparison.Ordinal);
    }

    [Fact]
    public async Task MailKitEngineAddAccountUsesProviderDefaults()
    {
        using var engine = MailKitEngine.OpenInMemory();
        var acc = await engine.AddAccountAsync(new AddAccountRequest { Email = "dev@qq.com" });
        Assert.Equal("qq", acc.ProviderId);
        var folders = await engine.ListFoldersAsync(acc.Id);
        Assert.Empty(folders);
        var msgs = await engine.ListMessagesAsync("missing");
        Assert.Empty(msgs);
        await Assert.ThrowsAsync<EngineException>(() => engine.GetBodyAsync("m"));
        var draft = await engine.SendAsync(new SendRequest { AccountId = acc.Id, To = ["bob@imyemail.test"], UndoWindowSecs = 10 });
        await engine.UndoSendAsync(draft);
        Assert.Equal(0u, await engine.FlushDueSendsAsync());
        var providers = await engine.ListProvidersAsync();
        Assert.Equal(20, providers.Count);
    }

    [Fact]
    public async Task MailKitEngineRejectsInvalidEmail()
    {
        using var engine = MailKitEngine.OpenInMemory();
        var ex = await Assert.ThrowsAsync<EngineException>(() => engine.AddAccountAsync(new AddAccountRequest { Email = "not-an-email" }));
        Assert.Equal(EngineErrorKind.Invalid, ex.Kind);
    }

    [Fact]
    public void FolderRoleFromSpecialUseAndChinese()
    {
        Assert.Equal("sent", FolderRoles.FromFlagsAndName(["sent"], "whatever"));
        Assert.Equal("sent", FolderRoles.FromFlagsAndName([], "已发送"));
        Assert.Equal("inbox", FolderRoles.FromFlagsAndName([], "INBOX"));
    }

    private static void AssertSecretsAbsentFromDatabase(string databasePath, params string[] secrets)
    {
        Assert.True(File.Exists(databasePath));
        // SQLite keeps writable handles open on Windows; share access without closing the
        // store, and inspect the WAL too because recent account writes can live there.
        foreach (var path in new[] { databasePath, databasePath + "-wal" })
        {
            if (!File.Exists(path)) continue;
            using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
            using var reader = new StreamReader(stream);
            var raw = reader.ReadToEnd();
            foreach (var secret in secrets)
                Assert.DoesNotContain(secret, raw, StringComparison.Ordinal);
        }
    }
}
