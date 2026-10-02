import ChckDesign
import Foundation

public enum MarkdownFormatting: String, CaseIterable, Identifiable, Sendable {
    case heading, bold, italic, link, bullet, numbered, quote, code
    public var id: Self { self }
    public var title: String {
        switch self {
        case .heading: L10n.t("标题")
        case .bold: L10n.t("粗体")
        case .italic: L10n.t("斜体")
        case .link: L10n.t("链接")
        case .bullet: L10n.t("无序列表")
        case .numbered: L10n.t("有序列表")
        case .quote: L10n.t("引用")
        case .code: L10n.t("代码")
        }
    }
    public var symbol: String {
        switch self {
        case .heading: "textformat.size"
        case .bold: "bold"
        case .italic: "italic"
        case .link: "link"
        case .bullet: "list.bullet"
        case .numbered: "list.number"
        case .quote: "text.quote"
        case .code: "chevron.left.forwardslash.chevron.right"
        }
    }

    /// NSRange uses UTF-16, matching NSTextView even for emoji and Chinese selections.
    public func edit(_ source: String, selection: NSRange) -> (range: NSRange, replacement: String, selection: NSRange) {
        let text = source as NSString
        let start = min(max(0, selection.location), text.length)
        var range = NSRange(location: start, length: min(max(0, selection.length), text.length - start))
        if [.heading, .bullet, .numbered, .quote].contains(self) {
            range = text.lineRange(for: range)
            let selected = text.substring(with: range)
            var lines = selected.components(separatedBy: "\n")
            let trailingNewline = lines.count > 1 && lines.last == ""
            if trailingNewline { lines.removeLast() }
            let prefix: String = switch self {
            case .heading: "## "
            case .quote: "> "
            case .numbered: "1. "
            default: "- "
            }
            let replacement = lines.enumerated().map { index, line in
                (self == .numbered ? "\(index + 1). " : prefix) + line
            }.joined(separator: "\n") + (trailingNewline ? "\n" : "")
            let selectedRange = range.length == 0
                ? NSRange(location: range.location + (prefix as NSString).length, length: 0)
                : NSRange(location: range.location, length: (replacement as NSString).length)
            return (range, replacement, selectedRange)
        }
        let selected = text.substring(with: range)
        let content = selected.isEmpty ? title : selected
        let prefix: String
        let suffix: String
        switch self {
        case .bold: (prefix, suffix) = ("**", "**")
        case .italic: (prefix, suffix) = ("*", "*")
        case .link: (prefix, suffix) = ("[", "](https://example.com)")
        case .code:
            if content.contains("\n") {
                let fence = String(repeating: "`", count: max(3, longestBacktickRun(content) + 1))
                (prefix, suffix) = ("\n\(fence)\n", "\n\(fence)\n")
            } else {
                let fence = String(repeating: "`", count: longestBacktickRun(content) + 1)
                (prefix, suffix) = (fence + " ", " " + fence)
            }
        default: (prefix, suffix) = ("", "")
        }
        let replacement = prefix + content + suffix
        let cursor = self == .link
            ? NSRange(location: range.location + (prefix + content + "](" as NSString).length, length: ("https://example.com" as NSString).length)
            : NSRange(location: range.location + (prefix as NSString).length, length: (content as NSString).length)
        return (range, replacement, cursor)
    }

    private func longestBacktickRun(_ text: String) -> Int {
        var longest = 0
        var run = 0
        for character in text {
            run = character == "`" ? run + 1 : 0
            longest = max(longest, run)
        }
        return longest
    }
}
