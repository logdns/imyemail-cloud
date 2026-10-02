using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Xunit;

namespace Chck.Mail.Tests;

public class UidValidityTests
{
    [Theory]
    [InlineData(null)]
    [InlineData(10u)]
    public async Task BodyFetchRejectsUnknownOrChangedMailboxBeforeFetchingMessage(uint? previousValidity)
    {
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        using var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        using var key = RSA.Create(2048);
        var request = new CertificateRequest("CN=localhost", key, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);
        using var generatedCertificate = request.CreateSelfSigned(DateTimeOffset.UtcNow.AddMinutes(-1), DateTimeOffset.UtcNow.AddDays(1));
        // Windows Schannel cannot use the ephemeral RSA key returned by CreateSelfSigned.
        // Import a user key container for this fixture; disposal removes it because we do
        // not request PersistKeySet. No certificate is added to a trusted certificate store.
        using var certificate = X509CertificateLoader.LoadPkcs12(
            generatedCertificate.Export(X509ContentType.Pkcs12), null,
            OperatingSystem.IsWindows() ? X509KeyStorageFlags.UserKeySet : X509KeyStorageFlags.DefaultKeySet);
        var commands = new List<string>();
        var server = Task.Run(async () =>
        {
            using var client = await listener.AcceptTcpClientAsync(cancellation.Token);
            using var tls = new SslStream(client.GetStream());
            await tls.AuthenticateAsServerAsync(new SslServerAuthenticationOptions { ServerCertificate = certificate }, cancellation.Token);
            using var reader = new StreamReader(tls, Encoding.ASCII);
            using var writer = new StreamWriter(tls, Encoding.ASCII) { AutoFlush = true, NewLine = "\r\n" };
            await writer.WriteLineAsync("* OK [CAPABILITY IMAP4rev1 AUTH=PLAIN SASL-IR] test server");
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
                    await writer.WriteLineAsync("* FLAGS (\\Seen)");
                    await writer.WriteLineAsync("* 1 EXISTS");
                    await writer.WriteLineAsync("* OK [UIDVALIDITY 20] changed mailbox");
                    await writer.WriteLineAsync("* OK [UIDNEXT 2] next UID");
                }
                await writer.WriteLineAsync(tag + " OK completed");
            }
        });
        var directory = Path.Combine(Path.GetTempPath(), "chck-win-validity-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, "mail.db");
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        var account = await engine.AddAccountAsync(new AddAccountRequest
        {
            Email = "test@example.test",
            Password = "test-only",
            ImapHost = "127.0.0.1",
            ImapPort = (ushort)((IPEndPoint)listener.LocalEndpoint).Port,
            AcceptInvalidCerts = true,
        });
        var folderId = account.Id + ":INBOX";
        using (var store = new MailStore(path))
        {
            store.UpsertFolder(new Folder(folderId, "INBOX", "inbox", 1), account.Id);
            if (previousValidity is { } value) store.SetFolderUidValidity(folderId, value);
            store.UpsertMessage(account.Id, folderId, 1, "Original", "[]", 0, "", new MailFlags(), false, null, null);
        }
        var messageId = Assert.Single(await engine.ListMessagesAsync(folderId)).Id;
        var error = await Assert.ThrowsAsync<EngineException>(() => engine.GetBodyAsync(messageId, cancellation.Token));
        Assert.Equal(EngineErrorKind.Invalid, error.Kind);
        Assert.Contains("同步", error.Message);
        await server.WaitAsync(cancellation.Token);
        Assert.Contains(commands, command => command.Contains(" EXAMINE ", StringComparison.Ordinal));
        Assert.DoesNotContain(commands, command => command.Contains("FETCH", StringComparison.OrdinalIgnoreCase));
        Assert.Null(await engine.GetCachedBodyAsync(messageId));
    }
}
