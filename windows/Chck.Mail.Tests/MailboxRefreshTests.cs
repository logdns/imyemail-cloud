using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class MailboxRefreshTests
{
    [Fact]
    public async Task LateBodyAndAttachmentsNeverReplaceNewSelection()
    {
        var engine = new ControlledMailEngine();
        var started = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var slow = new TaskCompletionSource<MessageBody>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Fetch = id =>
        {
            if (id != "a:INBOX:1") return Task.FromResult(new MessageBody(id, ""));
            started.TrySetResult();
            return slow.Task;
        };
        var vm = new MailboxViewModel(engine);
        var first = vm.OpenAsync(ControlledMailEngine.Row("a:INBOX:1"));
        await started.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await vm.OpenAsync(ControlledMailEngine.Row("b:INBOX:1"));
        slow.SetResult(new("old", ""));
        await first;
        Assert.Equal("b:INBOX:1", vm.Body?.Text);
        Assert.Equal("b:INBOX:1", vm.SelectedMessage?.Id);
        Assert.Equal(1, engine.FlagWrites);

        var attachmentStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var attachments = new TaskCompletionSource<IReadOnlyList<AttachmentHandle>>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Attachments = id =>
        {
            if (id != "a:INBOX:1") return Task.FromResult<IReadOnlyList<AttachmentHandle>>([]);
            attachmentStarted.TrySetResult();
            return attachments.Task;
        };
        first = vm.OpenAsync(ControlledMailEngine.Row("a:INBOX:1"));
        await attachmentStarted.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await vm.OpenAsync(ControlledMailEngine.Row("b:INBOX:1"));
        attachments.SetResult([new("old", "old.txt", "text/plain", 1, null)]);
        await first;
        Assert.Equal("b:INBOX:1", vm.Body?.Text);
        Assert.Empty(vm.Attachments);
    }

    [Fact]
    public async Task PreloadsAdjacentBodiesWithoutChangingUnreadFlags()
    {
        var engine = new ControlledMailEngine();
        var loaded = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var count = 0;
        engine.Fetch = id =>
        {
            if (Interlocked.Increment(ref count) == 3) loaded.TrySetResult();
            return Task.FromResult(new MessageBody(id, ""));
        };
        var vm = new MailboxViewModel(engine)
        {
            Messages = [ControlledMailEngine.Row("a:INBOX:1"), ControlledMailEngine.Row("a:INBOX:2"), ControlledMailEngine.Row("a:INBOX:3")],
        };
        await vm.OpenAsync(vm.Messages[1]);
        await loaded.Task.WaitAsync(TimeSpan.FromSeconds(5));
        Assert.Equal(3, engine.Fetches.Count);
        Assert.All(vm.Messages.Where(row => row.Id != "a:INBOX:2"), row => Assert.True(row.Unread));
        Assert.False(vm.Messages[1].Unread);
        Assert.Equal(1, engine.FlagWrites);
        Assert.Equal("a:INBOX:2", vm.Body?.Text);
    }

    [Fact]
    public async Task EmptyBaselineThenFirstArrivalsNotifyAcrossAccountsWhileSearchIsPreserved()
    {
        var engine = new ControlledMailEngine();
        var vm = new MailboxViewModel(engine);
        var notifications = new List<(MessageRow Row, int Count)>();
        vm.NotificationArrived += (row, count) => notifications.Add((row, count));
        await vm.BootstrapAsync();
        Assert.Empty(notifications);
        engine.SearchRows = [ControlledMailEngine.Row("a:Archive:9")];
        vm.SelectedFolder = vm.Folders.First(folder => folder.Role == "archive");
        vm.SearchQuery = "report";
        await vm.SearchAsync();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1", date: 10)];
        engine.Rows["b:INBOX"] = [ControlledMailEngine.Row("b:INBOX:1", date: 20)];
        await vm.RefreshAsync();
        Assert.Equal(2, vm.UnreadCount);
        Assert.Equal(2, vm.NewMailCount);
        Assert.Equal("b:INBOX:1", vm.LatestArrival?.Id);
        Assert.Equal(2, Assert.Single(notifications).Count);
        Assert.Equal("a:Archive:9", Assert.Single(vm.Messages).Id);
        Assert.Contains("a:INBOX", engine.Synced);
        Assert.Contains("b:INBOX", engine.Synced);
        await vm.RefreshAsync();
        Assert.Single(notifications);
        vm.DismissNewMail();
        Assert.Equal(0, vm.NewMailCount);
        Assert.Null(vm.LatestArrival);
    }

    [Fact]
    public async Task InitialHistoryAndReadFlagChangesDoNotNotify()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1", unread: false)];
        var vm = new MailboxViewModel(engine);
        var count = 0;
        vm.NotificationArrived += (_, n) => count += n;
        await vm.BootstrapAsync();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1"), ControlledMailEngine.Row("a:INBOX:2", date: 2)];
        await vm.RefreshAsync();
        Assert.Equal(1, count);
        Assert.Equal("a:INBOX:2", vm.LatestArrival?.Id);
    }

    [Fact]
    public async Task SearchStartedDuringRefreshCannotBeOverwrittenByInbox()
    {
        var engine = new ControlledMailEngine();
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Sync = _ => { entered.TrySetResult(); return release.Task; };
        var refresh = vm.RefreshAsync();
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        vm.SearchQuery = "important";
        engine.SearchRows = [ControlledMailEngine.Row("a:Archive:9")];
        await vm.SearchAsync();
        release.SetResult();
        await refresh;
        Assert.Equal("a:Archive:9", Assert.Single(vm.Messages).Id);
    }

    [Fact]
    public async Task UnifiedInboxSelectionDuringRefreshReplacesPreviousFolder()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1")];
        engine.Rows["a:Archive"] = [ControlledMailEngine.Row("a:Archive:9")];
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.EngineSyncAsync("a:Archive");
        Assert.Equal("a:Archive:9", Assert.Single(vm.Messages).Id);

        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Sync = _ => { entered.TrySetResult(); return release.Task; };
        var refresh = vm.RefreshAsync();
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await vm.LoadUnifiedInboxAsync();
        Assert.Null(vm.SelectedFolder);
        Assert.Equal("a:INBOX:1", Assert.Single(vm.Messages).Id);
        release.SetResult();
        await refresh;
        Assert.Equal("a:INBOX:1", Assert.Single(vm.Messages).Id);
    }

    [Fact]
    public async Task NotificationNavigationValidatesAccountAndClearsSearch()
    {
        var engine = new ControlledMailEngine();
        engine.Rows["a:INBOX"] = [ControlledMailEngine.Row("a:INBOX:1")];
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenNotificationAsync("a:INBOX:1", "b");
        Assert.Null(vm.SelectedMessage);
        Assert.Empty(engine.Fetches);
        vm.SearchQuery = "search";
        await vm.OpenNotificationAsync("a:INBOX:1", "a");
        Assert.Equal("", vm.SearchQuery);
        Assert.Equal("a:INBOX", vm.SelectedFolder?.Id);
        Assert.Equal("a:INBOX:1", vm.SelectedMessage?.Id);
        Assert.Equal("a:INBOX:1", vm.Body?.Text);
    }

    [Fact]
    public async Task ClearFailedQueueReportsActualCount()
    {
        var engine = new ControlledMailEngine();
        var vm = new MailboxViewModel(engine);
        await vm.ClearFailedQueueAsync();
        Assert.Equal(1, engine.ClearCalls);
        Assert.Contains("3", vm.Status);
    }
}
