import ChckAppCore
import SwiftUI
import WebKit

/// One rendering and navigation policy for both Apple platforms.
struct MailWebView {
    var html: String
    var allowRemote: Bool
    @Environment(\.colorScheme) private var colorScheme

    @MainActor
    func makeView(coordinator: Coordinator) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        configuration.websiteDataStore = .nonPersistent()
        let view = WKWebView(frame: .zero, configuration: configuration)
        #if os(macOS)
        view.setValue(false, forKey: "drawsBackground")
        #else
        view.isOpaque = false
        view.backgroundColor = .clear
        #endif
        view.navigationDelegate = coordinator
        return view
    }

    @MainActor
    func update(_ view: WKWebView, coordinator: Coordinator, resolvedColorScheme: ColorScheme? = nil) {
        let scheme = resolvedColorScheme ?? colorScheme
        let document = MailContentPolicy.document(html: html, allowRemote: allowRemote, colorScheme: scheme == .dark ? "dark" : "light")
        guard coordinator.document != document else { return }
        coordinator.document = document
        view.loadHTMLString(document, baseURL: nil)
    }

    @MainActor
    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate {
        var document: String?

        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
            guard let url = navigationAction.request.url else { return .cancel }
            if navigationAction.navigationType == .linkActivated {
                if MailContentPolicy.allowsExternalURL(url) {
                    #if os(macOS)
                    NSWorkspace.shared.open(url)
                    #else
                    await UIApplication.shared.open(url)
                    #endif
                }
                return .cancel
            }
            // loadHTMLString uses about:blank. Cancel redirects and unsolicited navigation.
            return url.absoluteString == "about:blank" ? .allow : .cancel
        }
    }
}

#if os(macOS)
extension MailWebView: NSViewRepresentable {
    func makeNSView(context: Context) -> WKWebView { makeView(coordinator: context.coordinator) }
    func updateNSView(_ view: WKWebView, context: Context) { update(view, coordinator: context.coordinator) }
}
#else
extension MailWebView: UIViewRepresentable {
    func makeUIView(context: Context) -> WKWebView { makeView(coordinator: context.coordinator) }
    func updateUIView(_ view: WKWebView, context: Context) { update(view, coordinator: context.coordinator) }
}
#endif
