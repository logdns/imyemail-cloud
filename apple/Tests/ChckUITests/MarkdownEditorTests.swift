#if os(macOS)
import AppKit
import ChckAppCore
@testable import ChckUI
import XCTest

final class MarkdownEditorTests: XCTestCase {
    @MainActor
    func testRemoteImagePreferenceReloadsTheExistingWebViewPolicy() {
        let blocked = MailWebView(
            html: #"<p>Mail</p><img src="https://images.example.test/mail.png">"#,
            allowRemote: false
        )
        let coordinator = blocked.makeCoordinator()
        let webView = blocked.makeView(coordinator: coordinator)
        blocked.update(webView, coordinator: coordinator, resolvedColorScheme: .light)
        let blockedDocument = coordinator.document
        XCTAssertTrue(blockedDocument?.contains("img-src data:") == true)

        let allowed = MailWebView(html: blocked.html, allowRemote: true)
        allowed.update(webView, coordinator: coordinator, resolvedColorScheme: .light)
        XCTAssertNotEqual(coordinator.document, blockedDocument)
        XCTAssertTrue(coordinator.document?.contains("img-src https: data:") == true)
        XCTAssertFalse(webView.configuration.defaultWebpagePreferences.allowsContentJavaScript)
    }

    @MainActor
    func testClosedMainWindowCanBeReopened() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 640, height: 400),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        MacMainWindowLifecycle.configure(window)
        window.orderFront(nil)
        window.close()

        XCTAssertFalse(window.isVisible)
        XCTAssertFalse(window.isReleasedWhenClosed)
        XCTAssertTrue(MacMainWindowLifecycle.reopen(in: [window]))
        XCTAssertTrue(window.isVisible)
        window.close()
    }

    @MainActor
    func testNativeSelectionFormattingAndUndoPreserveChineseAndEmoji() {
        let scroll = MarkdownNativeTextView.scrollableTextView()
        let view = scroll.documentView as! MarkdownNativeTextView
        view.isRichText = false
        view.allowsUndo = true
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        window.makeFirstResponder(view)
        let controller = MarkdownEditorController()
        controller.textView = view
        view.string = "你好 👋 world"
        view.setSelectedRange((view.string as NSString).range(of: "你好 👋"))
        controller.apply(.bold)
        XCTAssertEqual(view.string, "**你好 👋** world")
        view.undoManager?.undo()
        XCTAssertEqual(view.string, "你好 👋 world")
        view.undoManager?.redo()
        XCTAssertEqual(view.string, "**你好 👋** world")
    }

    @MainActor
    func testNativeMarkedTextCommitsWithoutDroppingInput() {
        let view = MarkdownNativeTextView()
        view.isRichText = false
        view.string = "正文 "
        view.setSelectedRange(NSRange(location: 3, length: 0))
        view.setMarkedText("zhongwen", selectedRange: NSRange(location: 8, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertTrue(view.hasMarkedText())
        view.insertText("中文", replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertFalse(view.hasMarkedText())
        XCTAssertEqual(view.string, "正文 中文")
    }
}
#endif
