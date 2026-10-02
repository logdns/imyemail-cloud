using System.Diagnostics;
using System.Text.Json;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;

internal static class ProtocolSmoke
{
    // Deliberately fixed to the isolated local test server; never accept arbitrary SMTP recipients.
    private const string TestHost = "192.168.31.8";

    public static async Task RunAsync(string directory)
    {
        var database = Path.Combine(directory, "mail.db");
        var recipientDatabase = Path.Combine(directory, "recipient.db");
        if (File.Exists(recipientDatabase)) throw new IOException("Refusing to replace recipient fixture database.");
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        var ct = cancellation.Token;
        // Only publicly documented, fictional fixture passwords are stored beside these isolated DBs.
        // This lets the actual UI reuse the same fixture without putting real credentials in a harness.
        using var engine = new MailKitEngine(database, SecretStore.OpenBeside(database));
        var account = await engine.AddAccountAsync(Request("dev", "devpass"), ct);
        var folders = await engine.ListFoldersAsync(account.Id, ct);
        var inbox = folders.Single(folder => folder.Role == "inbox");
        var timer = Stopwatch.StartNew();
        await engine.SyncFolderAsync(inbox.Id, ct);
        var syncMs = timer.Elapsed.TotalMilliseconds;
        var rows = await engine.ListMessagesAsync(inbox.Id, ct);
        Check(rows.Count > 0, "Seed at least one local message before protocol validation.");
        var row = rows[0];
        Check(await engine.GetCachedBodyAsync(row.Id, ct) is null, "Expected a cold body cache.");
        timer.Restart();
        var body = await engine.GetBodyAsync(row.Id, ct);
        var coldBodyMs = timer.Elapsed.TotalMilliseconds;
        Check(!string.IsNullOrWhiteSpace(body.Text) || !string.IsNullOrWhiteSpace(body.Html), "Cold IMAP body is empty.");
        timer.Restart();
        Check(await engine.GetCachedBodyAsync(row.Id, ct) == body, "Cached body does not match downloaded content.");
        var cachedBodyMs = timer.Elapsed.TotalMilliseconds;

        var subject = "Local protocol verification " + Guid.NewGuid().ToString("N");
        const string text = "Synthetic local-only SMTP message. No public recipient or relay.";
        var draftId = await engine.SendAsync(new SendRequest
        {
            AccountId = account.Id, To = ["alice@imyemail.test"], Subject = subject,
            BodyText = text, UndoWindowSecs = 1,
        }, ct);
        using (var store = new MailStore(database))
            Check(store.GetOutbox(draftId)?.State == "queued", "Send did not enter the undo queue.");
        await Task.Delay(1200, ct);
        timer.Restart();
        Check(await engine.FlushDueSendsAsync(ct) == 1, "Scheduled SMTP delivery failed.");
        var sendMs = timer.Elapsed.TotalMilliseconds;
        using (var store = new MailStore(database))
            Check(store.GetOutbox(draftId)?.State == "sent", "Delivered send was not marked sent.");

        using var recipient = new MailKitEngine(recipientDatabase, SecretStore.OpenBeside(recipientDatabase));
        var alice = await recipient.AddAccountAsync(Request("alice", "alicepass"), ct);
        var recipientFolders = await recipient.ListFoldersAsync(alice.Id, ct);
        var aliceInbox = recipientFolders.Single(folder => folder.Role == "inbox");
        await recipient.SyncFolderAsync(aliceInbox.Id, ct);
        var delivered = (await recipient.ListMessagesAsync(aliceInbox.Id, ct)).Single(message => message.Subject == subject);
        var received = await recipient.GetBodyAsync(delivered.Id, ct);
        Check(received.Text.Trim() == text, "Recipient body differs from queued message.");
        Console.WriteLine(JsonSerializer.Serialize(new
        {
            ok = true, mode = "local-protocol", database, recipientDatabase,
            server = TestHost, imaps = 3993, smtps = 3465, inboxMessages = rows.Count,
            syncMs = Math.Round(syncMs, 3), coldBodyMs = Math.Round(coldBodyMs, 3), cachedBodyMs = Math.Round(cachedBodyMs, 3),
            sendMs = Math.Round(sendMs, 3), queuedThenSent = true, recipientBodyMatched = true,
            account = "dev@imyemail.test", fixtureOnlyTlsException = true,
            scope = "Isolated GreenMail only; no public mail delivery. Sidecars contain fictional fixture passwords."
        }));
    }

    private static AddAccountRequest Request(string localPart, string password) => new()
    {
        Email = localPart + "@imyemail.test", DisplayName = localPart == "dev" ? "本地测试 · Dev" : "本地测试 · Alice",
        Username = localPart + "@imyemail.test", Password = password,
        ImapHost = TestHost, ImapPort = 3993, SmtpHost = TestHost, SmtpPort = 3465,
        AcceptInvalidCerts = true, ImapStartTls = false, SmtpStartTls = false,
    };

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
