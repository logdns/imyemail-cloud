using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public sealed class MarkdownComposerRendererTests
{
    [Fact]
    public void FormatsReplyParagraphsQuoteListTableAndCode()
    {
        var html = MarkdownComposerRenderer.Render("# 项目更新\n\n**重要** _斜体_ ~~删除~~\n下一行\n\n> 原邮件\n\n- 第一项\n- 第二项\n\n|姓名|状态|\n|---|---|\n|甲|完成|\n\n```csharp\nvar x = 1 < 2;\n```\n");
        Assert.Contains("<h1>项目更新</h1>", html);
        Assert.Contains("<strong>重要</strong>", html);
        Assert.Contains("<em>斜体</em>", html);
        Assert.Contains("<del>删除</del>", html);
        Assert.Contains("<br", html);
        Assert.Contains("<blockquote>", html);
        Assert.Contains("<li>第一项</li>", html);
        Assert.Contains("<table>", html);
        Assert.Contains("<td>完成</td>", html);
        Assert.Contains("var x = 1 &lt; 2;", html);
    }

    [Theory]
    [InlineData("[bad](javascript:alert%281%29)")]
    [InlineData("[bad](JaVaScRiPt:alert%281%29)")]
    [InlineData("[bad](data:text/html;base64,PHNjcmlwdD4=)")]
    [InlineData("[bad](file:///C:/private.txt)")]
    [InlineData("[bad](//tracker.test/a)")]
    [InlineData("[bad](javascript&#58;alert%281%29)")]
    [InlineData("<javascript:alert%281%29>")]
    [InlineData("<file:///C:/private.txt>")]
    public void UnsafeLinksCannotNavigate(string markdown)
    {
        var html = MarkdownComposerRenderer.Render(markdown);
        Assert.DoesNotContain("href=\"javascript", html, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("href=\"file", html, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("href=\"data", html, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("href=\"//", html);
    }

    [Fact]
    public void SafeLinksAndEmailAddressesRemainClickable()
    {
        var html = MarkdownComposerRenderer.Render("[docs](https://example.test/docs?q=1&b=2) [mail](mailto:user@example.test) <https://example.test> <user@example.test>");
        Assert.Contains("href=\"https://example.test/docs?q=1&amp;b=2\"", html);
        Assert.Contains("href=\"mailto:user@example.test\"", html);
        Assert.Contains("href=\"https://example.test\"", html);
    }

    [Fact]
    public void RawHtmlIsTextAndImagesOnlyKeepAltText()
    {
        var html = MarkdownComposerRenderer.Render("<img src=x onerror=alert(1)>\n<script>alert('x')</script>\n\n![diagram](https://tracker.test/pixel)\n\n<img src=\"file:///private\">");
        Assert.DoesNotContain("<img", html);
        Assert.DoesNotContain("<script", html);
        Assert.DoesNotContain("<iframe", html);
        Assert.DoesNotContain("https://tracker.test/pixel", html);
        Assert.Contains("diagram", html);
        Assert.Contains("&lt;", html);
    }

    [Fact]
    public void LinkTitlesAndCodeCannotInjectAttributesOrTags()
    {
        var html = MarkdownComposerRenderer.Render("[safe](https://example.test \"x &quot; onclick=&quot;alert(1)\")\n\n`<img src=x onerror=alert(1)>`");
        Assert.DoesNotContain("<img", html);
        Assert.DoesNotContain("\" onclick=\"", html);
        Assert.Contains("<code>&lt;img", html);
    }

    [Fact]
    public void CodeAndProseAreNotMistakenForActualEventAttributes()
    {
        var html = MarkdownComposerRenderer.Render("`<button onclick='sample()'>`\n\nExplain onclick='example' in prose.");
        Assert.Contains("onclick='sample()'", html);
        Assert.Contains("onclick='example'", html);
        Assert.DoesNotContain("<button", html);
    }

    [Fact]
    public async Task CancelledPreviewDoesNotPublishResult()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => MarkdownComposerRenderer.RenderAsync("**draft**", cancellation.Token));
    }

    [Fact]
    public void VeryLargeDraftIsPreservedAsEscapedText()
    {
        var draft = new string('x', 1_000_001) + "<script>";
        var html = MarkdownComposerRenderer.Render(draft);
        Assert.StartsWith("<pre>", html);
        Assert.EndsWith("&lt;script&gt;</pre>", html);
        Assert.Equal(draft.Length + 17, html.Length);
    }
}
