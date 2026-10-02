using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Text.Json;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Xunit;

namespace Chck.Mail.Tests;

public class FolderReconciliationTests
{
    [Theory]
    [InlineData("complete")]
    [InlineData("missing-flags")]
    [InlineData("changing-uids")]
    [InlineData("empty")]
    public async Task SyncReconcilesOnlyCompleteStableRemoteSnapshots(string mode)
    {
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        using var key = RSA.Create(2048);
        var request = new CertificateRequest("CN=localhost", key, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);
        using var generated = request.CreateSelfSigned(DateTimeOffset.UtcNow.AddMinutes(-1), DateTimeOffset.UtcNow.AddDays(1));
        using var certificate = X509CertificateLoader.LoadPkcs12(generated.Export(X509ContentType.Pkcs12), null,
            OperatingSystem.IsWindows() ? X509KeyStorageFlags.UserKeySet : X509KeyStorageFlags.DefaultKeySet);
        var commands = new List<string>();
        var server = Task.Run(async () =>
        {
            using var client = await listener.AcceptTcpClientAsync(cancellation.Token);
            using var tls = new SslStream(client.GetStream());
            await tls.AuthenticateAsServerAsync(new SslServerAuthenticationOptions { ServerCertificate = certificate }, cancellation.Token);
            using var reader = new StreamReader(tls, Encoding.ASCII);
            using var writer = new StreamWriter(tls, Encoding.ASCII) { AutoFlush = true, NewLine = "\r\n" };
            await writer.WriteLineAsync("* OK [CAPABILITY IMAP4rev1 AUTH=PLAIN SASL-IR] fixture");
            var searches = 0;
            while (await reader.ReadLineAsync(cancellation.Token) is { } line)
            {
                commands.Add(line);
                var tag = line.Split(' ')[0];
                if (line.Contains(" CAPABILITY", StringComparison.Ordinal))
                    await writer.WriteLineAsync("* CAPABILITY IMAP4rev1 AUTH=PLAIN SASL-IR");
                else if (line.Contains(" LIST ", StringComparison.Ordinal))
                    await writer.WriteLineAsync("* LIST (\\Inbox) \"/\" \"INBOX\"");
                else if (line.Contains(" EXAMINE ", StringComparison.Ordinal))
                {
                    await writer.WriteLineAsync("* FLAGS (\\Seen \\Flagged)");
                    await writer.WriteLineAsync(mode == "empty" ? "* 0 EXISTS" : "* 2 EXISTS");
                    await writer.WriteLineAsync("* OK [UIDVALIDITY 10] known identity");
                    await writer.WriteLineAsync("* OK [UIDNEXT 3] next UID");
                    await writer.WriteLineAsync(tag + " OK [READ-ONLY] selected");
                    continue;
                }
                else if (line.Contains(" SEARCH ", StringComparison.Ordinal))
                {
                    searches++;
                    var both = mode == "missing-flags" || mode == "changing-uids" && searches > 1;
                    await writer.WriteLineAsync(mode == "empty" ? "* SEARCH" : both ? "* SEARCH 1 2" : "* SEARCH 1");
                }
                else if (line.Contains(" FETCH ", StringComparison.Ordinal) && !line.Contains("ENVELOPE", StringComparison.Ordinal))
                    await writer.WriteLineAsync("* 1 FETCH (UID 1 FLAGS (\\Seen \\Flagged))");
                else if (line.Contains(" LOGOUT", StringComparison.Ordinal))
                {
                    await writer.WriteLineAsync("* BYE fixture closing");
                    await writer.WriteLineAsync(tag + " OK logout");
                    break;
                }
                await writer.WriteLineAsync(tag + " OK complete");
            }
        });
        var path = Path.Combine(Path.GetTempPath(), "chck-reconcile-" + Guid.NewGuid().ToString("N") + ".db");
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        var account = await engine.AddAccountAsync(new AddAccountRequest
        {
            Email = "fixture@example.test",
            Password = "test-only",
            ImapHost = "127.0.0.1",
            ImapPort = (ushort)((IPEndPoint)listener.LocalEndpoint).Port,
            AcceptInvalidCerts = true,
        });
        var folder = account.Id + ":INBOX";
        using var store = new MailStore(path);
        store.UpsertFolder(new(folder, "INBOX", "inbox", 2), account.Id);
        store.SetFolderUidValidity(folder, 10);
        store.UpsertMessage(account.Id, folder, 1, "Survivor", "[]", 0, "Survivor", new(), false, null, null);
        store.UpsertMessage(account.Id, folder, 2, "Removed", "[]", 0, "Removed", new(), false, null, null);
        store.PutBody(folder + ":10:2", "<p>Removed body</p>", "Removed body");
        store.SetFolderCursor(folder, 3, 2, 2);
        if (mode is "complete" or "empty")
        {
            await engine.SyncFolderAsync(folder, cancellation.Token);
            if (mode == "complete")
            {
                var row = Assert.Single(store.ListMessages(folder));
                Assert.False(row.Unread);
                Assert.True(store.GetMessageFlags(row.Id)!.Flagged);
            }
            else Assert.Empty(store.ListMessages(folder));
            Assert.Null(store.GetBody(folder + ":10:2"));
            Assert.Empty(store.Search("Removed"));
            Assert.Equal(0u, Assert.Single(store.ListFolders(account.Id)).Unread);
        }
        else
        {
            await Assert.ThrowsAsync<EngineException>(() => engine.SyncFolderAsync(folder, cancellation.Token));
            Assert.Equal(2, store.ListMessages(folder).Count);
            Assert.All(store.ListMessages(folder), row => Assert.True(row.Unread));
            Assert.NotNull(store.GetBody(folder + ":10:2"));
        }
        await server.WaitAsync(cancellation.Token);
        Assert.DoesNotContain(commands, line => line.Contains("BODY[]", StringComparison.Ordinal) || line.Contains("BODY.PEEK[]", StringComparison.Ordinal));
    }

    [Fact]
    public void PendingLocalFlagsRemainOrderedAndOtherIdentityCannotOverrideSnapshot()
    {
        using var store = MailStore.OpenInMemory();
        store.InsertAccount(new("a", "a@example.test", "A", "custom"));
        store.UpsertFolder(new("a:INBOX", "INBOX", "inbox", 1), "a");
        store.SetFolderUidValidity("a:INBOX", 10);
        store.UpsertMessage("a", "a:INBOX", 1, "Flags", "[]", 0, "", new(), false, null, null);
        string Payload(uint validity, MailFlags flags) => JsonSerializer.Serialize(new { source = "INBOX", uidvalidity = validity, uid = 1, flags = flags.ToBits() });
        store.EnqueueOp("a", "flag", Payload(10, new MailFlags(Answered: true)));
        store.EnqueueOp("a", "mark_read", Payload(10, new MailFlags(Seen: true)));
        store.EnqueueOp("a", "flag", Payload(9, new MailFlags()));
        store.ReconcileFolderSnapshot("a:INBOX", 10, new Dictionary<uint, MailFlags> { [1] = new(Flagged: true) });
        Assert.Equal(new MailFlags(Seen: true, Answered: true), store.GetMessageFlags("a:INBOX:10:1"));
        Assert.Throws<EngineException>(() => store.ReconcileFolderSnapshot("a:INBOX", 9, new Dictionary<uint, MailFlags>()));
        Assert.Single(store.ListMessages("a:INBOX"));
    }
}
