using Markdig;
using Markdig.Renderers;
using Markdig.Renderers.Html.Inlines;
using Markdig.Syntax.Inlines;
using System.Text.RegularExpressions;

namespace Chck.Mail.Core;

/// <summary>One safe HTML representation shared by compose preview and MIME sending.</summary>
public static partial class MarkdownComposerRenderer
{
    private static readonly MarkdownPipeline Pipeline = new MarkdownPipelineBuilder()
        .DisableHtml()
        .UsePipeTables()
        .UseEmphasisExtras()
        .UseSoftlineBreakAsHardlineBreak()
        .Build();

    public static string Render(string markdown)
    {
        ArgumentNullException.ThrowIfNull(markdown);
        // Preserve unusually large drafts without expensive Markdown parsing.
        if (markdown.Length > 1_000_000) return "<pre>" + HtmlSanitizer.Escape(markdown) + "</pre>";
        var document = Markdown.Parse(markdown, Pipeline);
        using var writer = new StringWriter(System.Globalization.CultureInfo.InvariantCulture);
        var renderer = new HtmlRenderer(writer)
        {
            // Covers CommonMark autolinks as well as ordinary link renderers.
            LinkRewriter = url => IsSafeLink(url) ? url : "#"
        };
        Pipeline.Setup(renderer);
        renderer.ObjectRenderers.Replace<LinkInlineRenderer>(new ComposeLinkRenderer());
        renderer.Render(document);
        writer.Flush();
        // Raw HTML is disabled and Markdig escapes text/attributes. Apply the existing
        // sanitizer only to generated tags: its attribute regex must not rewrite code
        // samples or ordinary prose containing words such as "onclick='example'".
        return GeneratedTag().Replace(writer.ToString(), match => HtmlSanitizer.Sanitize(match.Value).Html);
    }

    public static Task<string> RenderAsync(string markdown, CancellationToken cancellationToken = default)
        => Task.Run(() =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            var html = Render(markdown);
            cancellationToken.ThrowIfCancellationRequested();
            return html;
        }, cancellationToken);

    private static bool IsSafeLink(string? url)
        => !string.IsNullOrWhiteSpace(url)
            && !url.Any(char.IsControl)
            && Uri.TryCreate(url, UriKind.Absolute, out var uri)
            && uri.Scheme is "http" or "https" or "mailto";

    [GeneratedRegex("<[^>]+>")]
    private static partial Regex GeneratedTag();

    private sealed class ComposeLinkRenderer : LinkInlineRenderer
    {
        protected override void Write(HtmlRenderer renderer, LinkInline link)
        {
            var url = link.GetDynamicUrl?.Invoke() ?? link.Url;
            if (link.IsImage || !IsSafeLink(url))
            {
                // Image alt text survives, but composing never inserts remote tracking images.
                renderer.WriteChildren(link);
                return;
            }
            base.Write(renderer, link);
        }
    }
}
