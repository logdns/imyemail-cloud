import ChckDesign
import Foundation
import Markdown

/// Shared by the composer preview and the outgoing HTML alternative.
/// Only generated tags and ordinary web/mail links are emitted. Raw HTML is text.
public enum MarkdownBody {
    public static func render(_ source: String) -> String {
        let document = Document(parsing: source)
        return "<div style=\"font-family:-apple-system,Arial,sans-serif;font-size:15px;line-height:1.65;overflow-wrap:anywhere\">\(html(document))</div>"
    }

    private static func escape(_ value: String) -> String {
        value.replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "<", with: "&lt;")
            .replacingOccurrences(of: ">", with: "&gt;")
            .replacingOccurrences(of: "\"", with: "&quot;")
            .replacingOccurrences(of: "'", with: "&#39;")
    }

    private static func html(_ node: any Markup) -> String {
        func children() -> String { node.children.map { html($0) }.joined() }
        switch node {
        case let text as Markdown.Text: return escape(text.string)
        case let code as InlineCode: return "<code>\(escape(code.code))</code>"
        case let code as CodeBlock:
            return "<pre style=\"white-space:pre-wrap;padding:12px;border:1px solid #999;border-radius:6px\"><code>\(escape(code.code))</code></pre>"
        case let raw as InlineHTML: return escape(raw.rawHTML)
        case let raw as HTMLBlock: return "<pre>\(escape(raw.rawHTML))</pre>"
        case is SoftBreak, is LineBreak: return "<br>\n"
        case is ThematicBreak: return "<hr>"
        case let heading as Heading: return "<h\(heading.level)>\(children())</h\(heading.level)>"
        case is Paragraph:
            let style = node.parent is ListItem ? " style=\"margin:0\"" : ""
            return "<p\(style)>\(children())</p>\n"
        case is Strong: return "<strong>\(children())</strong>"
        case is Emphasis: return "<em>\(children())</em>"
        case is Strikethrough: return "<del>\(children())</del>"
        case is BlockQuote:
            return "<blockquote style=\"border-left:3px solid #8ba8ff;margin-left:0;padding-left:16px\">\(children())</blockquote>"
        case is UnorderedList: return "<ul>\(children())</ul>"
        case let list as OrderedList: return "<ol start=\"\(list.startIndex)\">\(children())</ol>"
        case let item as ListItem:
            let checkbox = item.checkbox.map { $0 == .checked ? "☑ " : "☐ " } ?? ""
            return "<li>\(checkbox)\(children())</li>"
        case let link as Markdown.Link:
            guard let destination = link.destination, let url = URL(string: destination),
                  MailContentPolicy.allowsExternalURL(url) else { return children() }
            return "<a href=\"\(escape(destination))\">\(children())</a>"
        case let image as Markdown.Image:
            // Display images as explicit links: composing never loads tracking URLs or local files.
            let label = children().isEmpty ? L10n.t("图片") : children()
            guard let source = image.source, let url = URL(string: source),
                  ["https", "http"].contains(url.scheme?.lowercased() ?? ""), url.host != nil else { return label }
            return "<a href=\"\(escape(source))\">\(label)</a>"
        case is Table: return "<table style=\"border-collapse:collapse\">\(children())</table>"
        case is Table.Head: return "<thead><tr>\(children())</tr></thead>"
        case is Table.Body: return "<tbody>\(children())</tbody>"
        case is Table.Row: return "<tr>\(children())</tr>"
        case is Table.Cell:
            let tag = node.parent is Table.Head ? "th" : "td"
            return "<\(tag) style=\"border:1px solid #999;padding:6px 10px\">\(children())</\(tag)>"
        default: return children()
        }
    }
}
