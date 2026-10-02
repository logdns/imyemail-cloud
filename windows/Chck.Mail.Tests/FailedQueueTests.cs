using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Xunit;

namespace Chck.Mail.Tests;

public class FailedQueueTests
{
    [Fact]
    public async Task FailedSendLeavesActiveQueueAndCanBeRetainedAsDraft()
    {
        var directory = Path.Combine(Path.GetTempPath(), "chck-win-send-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, "mail.db");
        try
        {
            using var engine = new MailKitEngine(path, SecretStore.Memory());
            var account = await engine.AddAccountAsync(new AddAccountRequest { Email = "test@example.test" });
            using var store = new MailStore(path);
            // Missing SMTP configuration fails before any network connection.
            store.InsertOutbox("failed-send", account.Id,
                "{\"to\":[\"recipient@example.test\"],\"subject\":\"Preserve me\",\"body_text\":\"draft body\"}", "queued", 1);
            Assert.Equal(0u, await engine.FlushDueSendsAsync());
            Assert.Equal("failed", store.GetOutbox("failed-send")!.Value.State);
            Assert.Empty(store.DueOutbox(long.MaxValue));
            Assert.Equal(1, await engine.ClearFailedQueueAsync());
            var draft = store.GetOutbox("failed-send")!.Value;
            Assert.Equal("draft", draft.State);
            Assert.Contains("draft body", draft.Json);
        }
        finally
        {
            Microsoft.Data.Sqlite.SqliteConnection.ClearAllPools();
            Directory.Delete(directory, recursive: true);
        }
    }

    [Fact]
    public void ClearFailedQueueRetainsDraftContentAndNeverTouchesActiveSends()
    {
        using var store = MailStore.OpenInMemory();
        store.InsertAccount(new Account("a", "a@example.test", "A", "custom"));
        const string draft = "{\"subject\":\"Keep this draft\",\"body_text\":\"unsent body\"}";
        foreach (var state in new[] { "failed", "queued", "sending", "sent", "draft" })
        {
            store.InsertOutbox(state, "a", draft, state, 1);
        }
        Assert.Equal(1, store.ClearFailedQueue());
        Assert.Equal("draft", store.GetOutbox("failed")!.Value.State);
        Assert.Equal(draft, store.GetOutbox("failed")!.Value.Json);
        Assert.DoesNotContain("failed", store.DueOutbox(long.MaxValue));
        foreach (var state in new[] { "queued", "sending", "sent", "draft" })
        {
            Assert.Equal(state, store.GetOutbox(state)!.Value.State);
        }
        Assert.Equal(0, store.ClearFailedQueue());
    }
}
