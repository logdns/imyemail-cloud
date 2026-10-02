using Chck.Mail.Core;
using Chck.Mail.Data;
using Chck.Mail.MailKit;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Chck.Mail.Tests;

public class LocalReadingTests
{
    [Fact]
    public async Task UidResetDoesNotReuseCachedIdentityAndLegacyUpsertPreservesBody()
    {
        var path = Path.Combine(Path.GetTempPath(), "chck-identity-" + Guid.NewGuid().ToString("N") + ".db");
        using var store = new MailStore(path);
        store.InsertAccount(new("a", "a@example.test", "A", "custom"));
        store.UpsertFolder(new("a:INBOX", "INBOX", "inbox", 1), "a");
        store.UpsertMessage("a", "a:INBOX", 1, "Legacy", "[]", 0, "Legacy", new(), false, null, null);
        store.PutBody("a:INBOX:1", "<p>Old</p>", "Old");
        // Simulate an existing version's known-UIDVALIDITY row with its original ID.
        using (var connection = new SqliteConnection($"Data Source={path}"))
        {
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "UPDATE folders SET uidvalidity = 10 WHERE id = 'a:INBOX'";
            command.ExecuteNonQuery();
        }
        store.UpsertMessage("a", "a:INBOX", 1, "Updated", "[]", 0, "Updated", new(), false, null, null);
        Assert.Equal("a:INBOX:1", Assert.Single(store.ListMessages("a:INBOX")).Id);
        Assert.Equal("Old", store.GetBody("a:INBOX:1")?.Text);
        Assert.Equal("a:INBOX:1", Assert.Single(store.Search("Updated")).Id);
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        var cache = new MailBodyCache(engine);
        Assert.Equal("Old", (await cache.LoadAsync("a:INBOX:1")).Text);

        store.SetFolderUidValidity("a:INBOX", 20);
        store.UpsertMessage("a", "a:INBOX", 1, "Replacement", "[]", 0, "Replacement", new(), false, null, null);
        var replacement = Assert.Single(store.ListMessages("a:INBOX")).Id;
        Assert.Equal("a:INBOX:20:1", replacement);
        store.PutBody(replacement, "<p>New</p>", "New");
        Assert.Null(store.GetMessageMeta("a:INBOX:1"));
        Assert.Equal("New", (await cache.LoadAsync(replacement)).Text);
    }

    [Fact]
    public async Task BootstrapAndFolderSelectionDisplayLocalRowsDuringBlockedSync()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1")];
        engine.Rows["a:Archive"] = [ControlledMailEngine.Row("a:Archive:2")];
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Sync = _ => release.Task;
        var vm = new MailboxViewModel(engine);
        try
        {
            await vm.BootstrapAsync().WaitAsync(TimeSpan.FromSeconds(2));
            Assert.Equal("a:INBOX:1", Assert.Single(vm.Messages).Id);
            var folder = vm.EngineSyncAsync("a:Archive");
            Assert.False(folder.IsCompleted);
            Assert.Equal("a:Archive:2", Assert.Single(vm.Messages).Id);
            release.SetResult();
            await folder;
        }
        finally { release.TrySetResult(); }
    }

    [Fact]
    public async Task OpenMarksOnlyDisplayedMessageReadWithoutWaitingForWrite()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1"), ControlledMailEngine.Row("a:INBOX:2")];
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.MarkRead = _ => release.Task;
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        try
        {
            await vm.OpenAsync(vm.Messages[0]).WaitAsync(TimeSpan.FromSeconds(2));
            Assert.Equal("a:INBOX:1", vm.Body?.Text);
            Assert.False(vm.SelectedMessage!.Unread);
            Assert.False(vm.Messages[0].Unread);
            Assert.True(vm.Messages[1].Unread);
            Assert.Equal(1, vm.UnreadCount);
            Assert.Equal(1, engine.FlagWrites);
        }
        finally { release.TrySetResult(); }
    }

    [Fact]
    public async Task MarkReadPreservesAllOtherFlagsAndUpdatesFolderCount()
    {
        var path = Path.Combine(Path.GetTempPath(), "chck-read-" + Guid.NewGuid().ToString("N") + ".db");
        using var store = new MailStore(path);
        store.InsertAccount(new("a", "a@example.test", "A", "custom"));
        store.UpsertFolder(new("a:INBOX", "INBOX", "inbox", 1), "a");
        store.SetFolderUidValidity("a:INBOX", 10);
        var original = new MailFlags(Flagged: true, Answered: true, Draft: true, Deleted: true);
        store.UpsertMessage("a", "a:INBOX", 1, "Flags", "[]", 0, "", original, false, null, null);
        var id = Assert.Single(store.ListMessages("a:INBOX")).Id;
        using var engine = new MailKitEngine(path, SecretStore.Memory());
        await engine.MarkReadAsync(id);
        Assert.Equal(original with { Seen = true }, store.GetMessageFlags(id));
        Assert.Equal(0u, Assert.Single(store.ListFolders("a")).Unread);
        Assert.Single(store.ListPendingOps());
        await engine.MarkReadAsync(id);
        Assert.Single(store.ListPendingOps());
    }

    [Fact]
    public async Task FailedReadWriteRestoresUnreadWithoutClearingDisplayedBody()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1")];
        engine.MarkRead = _ => Task.FromException(new IOException("read write failed"));
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenAsync(vm.Messages[0]);
        Assert.Equal("a:INBOX:1", vm.Body?.Text);
        Assert.True(vm.SelectedMessage!.Unread);
        Assert.True(Assert.Single(vm.Messages).Unread);
        Assert.Equal(1, vm.UnreadCount);
        Assert.Contains("read write failed", vm.Status);
    }

    [Fact]
    public async Task RefreshRemovesReaderForReplacedIdentity()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:10:1")];
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenAsync(vm.Messages[0]);
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:20:1")];
        await vm.RefreshAsync();
        Assert.Null(vm.SelectedMessage);
        Assert.Null(vm.Body);
        await vm.OpenAsync(vm.Messages[0]);
        Assert.Equal("a:INBOX:20:1", vm.Body?.Text);
    }
}
