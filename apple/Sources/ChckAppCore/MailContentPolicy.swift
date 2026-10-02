import Foundation

/// Only deliberate, ordinary web/mail links may leave the isolated message renderer.
public enum MailContentPolicy {
    public static func allowsExternalURL(_ url: URL) -> Bool {
        switch url.scheme?.lowercased() {
        case "https", "http": return url.host?.isEmpty == false
        case "mailto": return true
        default: return false
        }
    }

    public static func document(html: String, allowRemote: Bool, colorScheme: String? = nil) -> String {
        let scheme = colorScheme == "dark" ? "dark" : colorScheme == "light" ? "light" : "light dark"
        let images = allowRemote ? "https: data:" : "data:"
        return """
        <!DOCTYPE html><html><head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src \(images); style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-src 'none';">
        <style>
        :root { color-scheme: \(scheme); }
        body { font: 14px -apple-system, BlinkMacSystemFont, sans-serif; line-height: 1.75; margin: 0 auto; padding: 14px 32px 28px; max-width: 760px; overflow-wrap: anywhere; color: CanvasText; background: transparent; }
        img { max-width: 100%; height: auto; }
        a { color: #3D6BFE; }
        blockquote { border-left: 3px solid #8ba8ff; margin-left: 0; padding-left: 16px; color: GrayText; }
        pre { white-space: pre-wrap; }
        </style></head><body>\(html)</body></html>
        """
    }
}
