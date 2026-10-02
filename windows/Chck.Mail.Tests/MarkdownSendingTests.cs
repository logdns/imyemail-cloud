using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public sealed class MarkdownSendingTests
{
    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task ViewModelQueuesFormattedHtmlOnlyWhenMarkdownIsEnabled(bool markdownEnabled)
    {
        const string draft = "## 进度更新\n\n**已经完成**\n\n> 原邮件内容\n\n[文档](https://example.test/docs)\n\n<script>unsafe()</script>";
        var engine = new ControlledMailEngine();
        var viewModel = new MailboxViewModel(engine);
        await viewModel.BootstrapAsync();
        Assert.True(viewModel.BeginCompose());
        viewModel.ComposeAccountId = "b";
        viewModel.ComposeTo = "recipient@example.test";
        viewModel.ComposeSubject = "Markdown sending integration";
        viewModel.ComposeMarkdown = markdownEnabled;
        viewModel.ComposeBody = draft;

        await viewModel.SendAsync();

        var queued = Assert.Single(engine.Sent);
        Assert.Equal("b", queued.AccountId);
        Assert.Equal("recipient@example.test", Assert.Single(queued.To));
        Assert.Equal(draft, queued.BodyText);
        if (markdownEnabled)
        {
            Assert.NotNull(queued.BodyHtml);
            Assert.Contains("<h2>进度更新</h2>", queued.BodyHtml);
            Assert.Contains("<strong>已经完成</strong>", queued.BodyHtml);
            Assert.Contains("<blockquote>", queued.BodyHtml);
            Assert.Contains("href=\"https://example.test/docs\"", queued.BodyHtml);
            Assert.DoesNotContain("<script>", queued.BodyHtml);
        }
        else Assert.Null(queued.BodyHtml);
        Assert.False(viewModel.ShowCompose);
        Assert.False(viewModel.IsSending);
        Assert.Empty(viewModel.ComposeBody);
    }
}
