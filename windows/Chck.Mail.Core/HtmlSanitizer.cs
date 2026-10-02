using System.Text.RegularExpressions;

namespace Chck.Mail.Core;

public sealed record SanitizeResult(string Html, uint RemoteBlocked);

public static partial class HtmlSanitizer
{
    public static SanitizeResult Sanitize(string html)
    {
        var remoteBlocked = CountRemote(html);
        var stripped = StripDangerous(html);
        stripped = StripRemoteSrc(stripped);
        return new SanitizeResult(stripped, remoteBlocked);
    }

    public static string Escape(string text) =>
        text.Replace("&", "&amp;", StringComparison.Ordinal)
            .Replace("<", "&lt;", StringComparison.Ordinal)
            .Replace(">", "&gt;", StringComparison.Ordinal);

    private static uint CountRemote(string html) =>
        (uint)(html.Split("src=\"http").Length - 1
               + html.Split("src='http").Length - 1
               + html.Split("url(http").Length - 1);

    private static string StripDangerous(string html)
    {
        var withoutScripts = ScriptTag().Replace(html, "");
        withoutScripts = EventAttr().Replace(withoutScripts, "");
        withoutScripts = JavascriptUri().Replace(withoutScripts, "href=\"#\"");
        withoutScripts = IframeTag().Replace(withoutScripts, "");
        withoutScripts = ObjectTag().Replace(withoutScripts, "");
        return withoutScripts;
    }

    private static string StripRemoteSrc(string html)
    {
        var outHtml = html;
        foreach (var needle in new[] { "src=\"http://", "src=\"https://", "src='http://", "src='https://" })
        {
            while (true)
            {
                var start = outHtml.IndexOf(needle, StringComparison.OrdinalIgnoreCase);
                if (start < 0)
                {
                    break;
                }

                var quote = needle[4];
                var rest = start + needle.Length;
                var endRel = outHtml.IndexOf(quote, rest);
                if (endRel < 0)
                {
                    break;
                }

                outHtml = string.Concat(outHtml.AsSpan(0, start), "data-chck-blocked=\"1\"", outHtml.AsSpan(endRel + 1));
            }
        }

        return outHtml;
    }

    [GeneratedRegex(@"<script[\s\S]*?</script>", RegexOptions.IgnoreCase)]
    private static partial Regex ScriptTag();

    [GeneratedRegex(@"\son\w+\s*=\s*(""[^""]*""|'[^']*')", RegexOptions.IgnoreCase)]
    private static partial Regex EventAttr();

    [GeneratedRegex(@"href\s*=\s*[""']javascript:[^""']*[""']", RegexOptions.IgnoreCase)]
    private static partial Regex JavascriptUri();

    [GeneratedRegex(@"<iframe[\s\S]*?</iframe>", RegexOptions.IgnoreCase)]
    private static partial Regex IframeTag();

    [GeneratedRegex(@"<object[\s\S]*?</object>", RegexOptions.IgnoreCase)]
    private static partial Regex ObjectTag();
}
