using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class MailboxViewModelTests
{
    [Fact]
    public async Task PreviewBootstrapLoadsInbox()
    {
        var store = new MailboxViewModel(new PreviewMailEngine());
        await store.BootstrapAsync();
        Assert.Single(store.Accounts);
        Assert.Equal("inbox", store.Folders[0].Role);
        Assert.Equal("Hello from testkit", store.Messages[0].Subject);
        Assert.Equal(20, store.Providers.Count);
        Assert.Equal("qq", store.Providers.First(p => p.Id == "qq").Id);
        Assert.Null(store.SelectedFolder);
        await store.OpenAsync(store.Messages[0]);
        Assert.Equal("Hello from testkit", store.Body?.Text);
        Assert.Empty(store.Attachments);
    }

    [Fact]
    public async Task LoadUnifiedInboxClearsFolderSelection()
    {
        var store = new MailboxViewModel(new PreviewMailEngine());
        await store.BootstrapAsync();
        store.SelectedFolder = store.Folders[0];
        await store.EngineSyncAsync(store.Folders[0].Id);
        await store.LoadUnifiedInboxAsync();
        Assert.Null(store.SelectedFolder);
        Assert.Equal("Hello from testkit", store.Messages[0].Subject);
    }

    [Fact]
    public async Task SendAndAddAccountOnPreview()
    {
        var store = new MailboxViewModel(new PreviewMailEngine());
        await store.BootstrapAsync();
        Assert.True(store.BeginCompose());
        store.ComposeTo = "bob@imyemail.test";
        store.ComposeSubject = "Hi";
        store.ComposeBody = "hello";
        await store.SendAsync();
        Assert.Equal("queued", store.Status);
        Assert.False(store.ShowCompose);
        store.DraftEmail = "dev@qq.com";
        store.DraftPassword = "x";
        await store.AddAccountAsync();
        Assert.False(store.ShowAddAccount);
    }
}
