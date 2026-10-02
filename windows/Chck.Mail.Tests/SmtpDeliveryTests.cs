using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Microsoft.Data.Sqlite;
using MimeKit;
using Xunit;

namespace Chck.Mail.Tests;

public sealed class SmtpDeliveryTests
{
    [Theory]
    [InlineData(false, false, false)]
    [InlineData(true, false, false)]
    [InlineData(true, true, false)]
    [InlineData(true, true, true)]
    public async Task QueuedMessageDeliversActualMimeAlternativesAndEscapedSignature(bool html, bool signature, bool fullDocument)
    {
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(15));
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
            await writer.WriteLineAsync("220 localhost SMTP fixture");
            MimeMessage? received = null;
            while (await reader.ReadLineAsync(cancellation.Token) is { } line)
            {
                commands.Add(line.Split(' ')[0]);
                if (line.StartsWith("EHLO ", StringComparison.Ordinal))
                {
                    await writer.WriteLineAsync("250-localhost");
                    await writer.WriteLineAsync("250 AUTH PLAIN");
                }
                else if (line.StartsWith("AUTH PLAIN ", StringComparison.Ordinal)) await writer.WriteLineAsync("235 2.7.0 authenticated");
                else if (line.StartsWith("MAIL FROM:", StringComparison.Ordinal) || line.StartsWith("RCPT TO:", StringComparison.Ordinal))
                    await writer.WriteLineAsync("250 2.1.0 accepted");
                else if (line == "DATA")
                {
                    await writer.WriteLineAsync("354 send message");
                    var wire = new StringBuilder();
                    while (await reader.ReadLineAsync(cancellation.Token) is { } data && data != ".")
                        wire.Append(data.StartsWith("..", StringComparison.Ordinal) ? data[1..] : data).Append("\r\n");
                    using var stream = new MemoryStream(Encoding.ASCII.GetBytes(wire.ToString()));
                    received = await MimeMessage.LoadAsync(stream, cancellation.Token);
                    await writer.WriteLineAsync("250 2.0.0 queued");
                }
                else if (line == "QUIT")
                {
                    await writer.WriteLineAsync("221 2.0.0 closing");
                    return received ?? throw new InvalidOperationException("No message received");
                }
                else throw new InvalidOperationException("Unexpected SMTP command: " + line.Split(' ')[0]);
            }
            throw new InvalidOperationException("SMTP session ended without QUIT");
        });
        var path = Path.Combine(Path.GetTempPath(), "chck-smtp-" + Guid.NewGuid().ToString("N") + ".db");
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        var account = await engine.AddAccountAsync(new AddAccountRequest
        {
            Email = "sender@example.test",
            DisplayName = "测试发件人",
            Password = "fixture-only",
            SmtpHost = "127.0.0.1",
            SmtpPort = (ushort)((IPEndPoint)listener.LocalEndpoint).Port,
            AcceptInvalidCerts = true,
        });
        const string signatureText = "Regards <script>alert('signature')</script> & team\nSecond line";
        if (signature)
        {
            using var connection = new SqliteConnection($"Data Source={path}");
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "INSERT INTO signatures(id,account_id,name,body,is_default) VALUES('sig',$account,'Default',$body,1)";
            command.Parameters.AddWithValue("$account", account.Id);
            command.Parameters.AddWithValue("$body", signatureText);
            command.ExecuteNonQuery();
        }
        const string markdown = "Hello **世界**\n\n| Name | Value |\n| --- | --- |\n| One | Two |";
        var rendered = MarkdownComposerRenderer.Render(markdown);
        if (fullDocument) rendered = "<html><body>" + rendered + "</body></html>";
        var draft = await engine.SendAsync(new SendRequest
        {
            AccountId = account.Id,
            To = ["recipient@example.test"],
            Subject = "MIME delivery verification",
            BodyText = markdown,
            BodyHtml = html ? rendered : null,
            UndoWindowSecs = 3600,
        }, cancellation.Token);
        using var store = new MailStore(path);
        Assert.Equal("queued", store.GetOutbox(draft)!.Value.State);
        // Advance only this fixture's scheduled queue entry, then exercise the same
        // flush path as the app's background worker without sleeping an undo window.
        using (var connection = new SqliteConnection($"Data Source={path}"))
        {
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "UPDATE outbox SET scheduled_at = 0 WHERE id = $id";
            command.Parameters.AddWithValue("$id", draft);
            command.ExecuteNonQuery();
        }
        Assert.Equal(1u, await engine.FlushDueSendsAsync(cancellation.Token));
        var delivered = await server.WaitAsync(cancellation.Token);
        Assert.Equal("sent", store.GetOutbox(draft)!.Value.State);
        Assert.Equal("测试发件人", delivered.From.Mailboxes.Single().Name);
        Assert.Contains("DATA", commands);
        Assert.NotNull(delivered.TextBody);
        Assert.NotNull(delivered.Body);
        var plain = delivered.TextBody.Replace("\r\n", "\n", StringComparison.Ordinal);
        Assert.Equal(signature ? markdown + "\n\n" + signatureText : markdown, plain);
        if (!html)
        {
            Assert.Null(delivered.HtmlBody);
            Assert.Equal("text/plain", delivered.Body.ContentType.MimeType);
            return;
        }
        Assert.NotNull(delivered.HtmlBody);
        Assert.Equal("multipart/alternative", delivered.Body.ContentType.MimeType);
        Assert.Contains("<strong>世界</strong>", delivered.HtmlBody, StringComparison.Ordinal);
        Assert.Contains("<table>", delivered.HtmlBody, StringComparison.Ordinal);
        if (signature)
        {
            Assert.Contains("&lt;script&gt;", delivered.HtmlBody, StringComparison.Ordinal);
            Assert.Contains("&amp; team<br />Second line", delivered.HtmlBody, StringComparison.Ordinal);
            Assert.DoesNotContain("<script>", delivered.HtmlBody, StringComparison.Ordinal);
            if (fullDocument) Assert.EndsWith("Second line</p></body></html>", delivered.HtmlBody, StringComparison.Ordinal);
        }
        else Assert.Equal(rendered, delivered.HtmlBody.Replace("\r\n", "\n", StringComparison.Ordinal));
    }
}
