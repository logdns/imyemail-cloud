using Chck.Mail.Core;
using Xunit;

namespace Chck.Mail.Tests;

public class SanitizeTests
{
    [Fact]
    public void StripsScriptAndJavascriptUri()
    {
        var r = HtmlSanitizer.Sanitize("""<p>hi</p><script>alert(1)</script><a href="javascript:alert(1)">x</a>""");
        Assert.DoesNotContain("script", r.Html, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("javascript:", r.Html, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("hi", r.Html, StringComparison.Ordinal);
    }

    [Fact]
    public void BlocksRemoteImages()
    {
        var r = HtmlSanitizer.Sanitize("""<p>ok</p><img src="https://tracker.example/pixel.gif">""");
        Assert.Equal(1u, r.RemoteBlocked);
        Assert.DoesNotContain("https://tracker.example", r.Html, StringComparison.Ordinal);
        Assert.Contains("data-chck-blocked", r.Html, StringComparison.Ordinal);
    }

    [Fact]
    public void KeepsCidImages()
    {
        var r = HtmlSanitizer.Sanitize("""<img src="cid:part1">""");
        Assert.Contains("cid:part1", r.Html, StringComparison.Ordinal);
        Assert.Equal(0u, r.RemoteBlocked);
    }
}
