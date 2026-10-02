using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class PreviewEngineTests
{
    [Fact]
    public async Task PreviewHasInbox()
    {
        var engine = new PreviewMailEngine();
        var accounts = await engine.ListAccountsAsync();
        Assert.Equal("dev@imyemail.test", accounts[0].Email);
        var folders = await engine.ListFoldersAsync(accounts[0].Id);
        Assert.Equal("inbox", folders[0].Role);
        var msgs = await engine.ListMessagesAsync(folders[0].Id);
        Assert.Equal("Hello from testkit", msgs[0].Subject);
    }

    [Fact]
    public async Task ProvidersAddAndSend()
    {
        var engine = new PreviewMailEngine();
        var providers = await engine.ListProvidersAsync();
        Assert.Contains(providers, p => p.Id == "qq" && p.ImapHost == "imap.qq.com");
        Assert.Equal(20, providers.Count);
        var acc = await engine.AddAccountAsync(new AddAccountRequest { Email = "dev@qq.com", Password = "x" });
        Assert.Equal("qq", acc.ProviderId);
        await engine.SendAsync(new SendRequest { AccountId = acc.Id, To = ["bob@imyemail.test"], Subject = "Hi", BodyText = "hello" });
    }

    [Fact]
    public async Task InvalidEmailRejected()
    {
        var engine = new PreviewMailEngine();
        await Assert.ThrowsAsync<EngineException>(() => engine.AddAccountAsync(new AddAccountRequest { Email = "not-an-email" }));
    }
}
