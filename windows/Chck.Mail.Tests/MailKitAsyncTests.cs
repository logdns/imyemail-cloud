using System.Net;
using System.Net.Sockets;
using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Xunit;

namespace Chck.Mail.Tests;

public sealed class MailKitAsyncTests
{
    [Fact]
    public async Task DiskCacheBypassesBlockedNetworkWithoutMarkingRead()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-async-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        var path = Path.Combine(dir, "mail.db");
        using (var store = new MailStore(path))
        {
            store.InsertAccount(new Account("a", "test@example.test", "Test", "custom"));
            store.UpsertFolder(new Folder("a:INBOX", "INBOX", "inbox", 1), "a");
            store.UpsertMessage("a", "a:INBOX", 1, "Cached", "[]", 0, "Cached", new MailFlags(), false, null, null);
            store.PutBody("a:INBOX:1", "<p>Cached body</p>", "Cached body");
        }
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        using var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        var probe = engine.TestAccountAsync(new AddAccountRequest
        {
            Email = "test@example.test",
            ImapHost = "127.0.0.1",
            ImapPort = (ushort)((IPEndPoint)listener.LocalEndpoint).Port,
            Password = "test-only",
            AcceptInvalidCerts = true,
        }, cancellation.Token);
        // The method must return before the peer completes even its TLS handshake.
        Assert.False(probe.IsCompleted);
        using var peer = await listener.AcceptTcpClientAsync().WaitAsync(TimeSpan.FromSeconds(2));

        var cached = await engine.GetCachedBodyAsync("a:INBOX:1").WaitAsync(TimeSpan.FromSeconds(2));
        Assert.Equal("Cached body", cached?.Text);
        Assert.Null(await engine.GetCachedBodyAsync("missing").WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Equal(cached, await engine.GetBodyAsync("a:INBOX:1").WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.False(probe.IsCompleted);

        using var queuedCancellation = new CancellationTokenSource();
        Assert.Single(await engine.ListAccountsAsync().WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Single(await engine.ListCachedFoldersAsync("a").WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Single(await engine.ListMessagesAsync("a:INBOX").WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Single(await engine.UnifiedInboxAsync().WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Single(await engine.SearchAsync("Cached").WaitAsync(TimeSpan.FromSeconds(2)));
        Assert.Empty(await engine.ListAttachmentsAsync("a:INBOX:1").WaitAsync(TimeSpan.FromSeconds(2)));
        var queued = engine.ClearFailedQueueAsync(queuedCancellation.Token);
        Assert.False(queued.IsCompleted);
        queuedCancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => queued);
        cancellation.Cancel();
        peer.Close();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => probe);
        var rows = await engine.ListMessagesAsync("a:INBOX");
        Assert.True(Assert.Single(rows).Unread);
    }

    [Fact]
    public async Task MemoryEngineSerializesConnectionAccessAndHonorsCancellation()
    {
        using var engine = MailKitEngine.OpenInMemory();
        await Task.WhenAll(Enumerable.Range(0, 30).Select(i => engine.AddAccountAsync(new AddAccountRequest
        {
            Email = $"user{i}@example.test",
        })));
        Assert.Equal(30, (await engine.ListAccountsAsync()).Count);
        var reads = Enumerable.Range(0, 30).Select(_ => engine.GetCachedBodyAsync("missing"));
        Assert.All(await Task.WhenAll(reads), body => Assert.Null(body));
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => engine.ListAccountsAsync(cancellation.Token));
        Assert.Equal(0, await engine.ClearFailedQueueAsync());
    }

    [Fact]
    public void VaultWriteFailureNeverCreatesPlaintextFallback()
    {
        var dir = Path.Combine(Path.GetTempPath(), "chck-win-vault-" + Guid.NewGuid().ToString("N"));
        var path = Path.Combine(dir, "mail.db");
        var overlay = new OverlaySecretStore(new UnavailableVault(), SecretStore.OpenBeside(path));
        Assert.Throws<InvalidOperationException>(() => overlay.Set("account", "test-only-secret"));
        Assert.False(File.Exists(path + ".secrets"));
        Assert.Null(overlay.Get("account"));
    }

    private sealed class UnavailableVault : ISecretStore
    {
        public string? Get(string id) => null;
        public void Set(string id, string secret) => throw new InvalidOperationException("vault unavailable");
        public void Delete(string id) { }
        public IReadOnlyDictionary<string, string> Snapshot() => new Dictionary<string, string>();
    }
}
