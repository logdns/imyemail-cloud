import ChckDesign
#if os(macOS)
import AppKit
import ChckAppCore
import SwiftUI

struct MarkdownComposer: View {
    @Binding var text: String
    @Environment(\.colorScheme) private var scheme
    @StateObject private var editor = MarkdownEditorController()
    @State private var mode: Mode = .edit
    @State private var previewHTML = ""
    private enum Mode: String, CaseIterable { case edit = "编辑", split = "分栏", preview = "预览" }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                ForEach(MarkdownFormatting.allCases) { format in
                    Button { editor.apply(format) } label: {
                        Image(systemName: format.symbol).frame(width: 24, height: 24)
                    }
                    .buttonStyle(.borderless)
                    .help(format.title)
                    .accessibilityLabel(format.title)
                    .disabled(mode == .preview)
                }
                Spacer(minLength: 8)
                Picker(L10n.t("正文显示"), selection: $mode) {
                    ForEach(Mode.allCases, id: \.self) { Text(L10n.t($0.rawValue)).tag($0) }
                }
                .labelsHidden()
                .pickerStyle(.segmented)
                .frame(width: 180)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .background(MailPalette(scheme: scheme).topbar)
            Divider()
            HStack(spacing: 0) {
                // Keep the native text view mounted to preserve selection and undo across preview changes.
                ZStack(alignment: .topLeading) {
                    MarkdownTextView(text: $text, controller: editor)
                    if text.isEmpty {
                        Text(L10n.t("用 Markdown 写信…"))
                            .foregroundStyle(.tertiary)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 16)
                            .allowsHitTesting(false)
                    }
                }
                .frame(maxWidth: mode == .preview ? 0 : .infinity)
                .clipped()
                .accessibilityHidden(mode == .preview)
                if mode != .edit {
                    if mode == .split { Divider() }
                    MailWebView(html: previewHTML, allowRemote: false)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .accessibilityLabel(L10n.t("Markdown 正文预览"))
                }
            }
            HStack {
                Text("Markdown")
                Text("·")
                Text(L10n.t("\(text.count) 字符"))
                Spacer()
                Text(L10n.t("发送时保留排版"))
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
        }
        .onChange(of: mode) { _, value in
            guard let view = editor.textView else { return }
            if value == .preview {
                view.window?.makeFirstResponder(nil)
            } else {
                view.window?.makeFirstResponder(view)
            }
        }
        .task(id: text) {
            let source = text
            do { try await Task.sleep(for: .milliseconds(150)) } catch { return }
            let html = await Task.detached(priority: .userInitiated) { MarkdownBody.render(source) }.value
            guard !Task.isCancelled else { return }
            previewHTML = html
        }
    }
}

@MainActor
final class MarkdownEditorController: ObservableObject {
    weak var textView: NSTextView?

    func apply(_ format: MarkdownFormatting) {
        guard let view = textView else { return }
        let edit = format.edit(view.string, selection: view.selectedRange())
        view.window?.makeFirstResponder(view)
        view.insertText(edit.replacement, replacementRange: edit.range)
        view.setSelectedRange(edit.selection)
        view.scrollRangeToVisible(edit.selection)
    }
}

@MainActor
final class MarkdownNativeTextView: NSTextView {
    var format: ((MarkdownFormatting) -> Void)?

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if window?.firstResponder === self,
           event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command {
            let action: MarkdownFormatting? = switch event.charactersIgnoringModifiers?.lowercased() {
            case "b": .bold
            case "i": .italic
            case "k": .link
            default: nil
            }
            if let action {
                format?(action)
                return true
            }
        }
        return super.performKeyEquivalent(with: event)
    }
}

private struct MarkdownTextView: NSViewRepresentable {
    @Binding var text: String
    let controller: MarkdownEditorController

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = MarkdownNativeTextView.scrollableTextView()
        let view = scroll.documentView as! MarkdownNativeTextView
        view.format = { [weak controller] in controller?.apply($0) }
        view.isRichText = false
        view.allowsUndo = true
        view.isAutomaticQuoteSubstitutionEnabled = false
        view.isAutomaticDashSubstitutionEnabled = false
        view.isAutomaticTextReplacementEnabled = false
        view.isAutomaticLinkDetectionEnabled = false
        view.font = .monospacedSystemFont(ofSize: 14, weight: .regular)
        view.textColor = .textColor
        view.backgroundColor = .textBackgroundColor
        view.textContainerInset = NSSize(width: 16, height: 16)
        view.textContainer?.widthTracksTextView = true
        view.isHorizontallyResizable = false
        view.autoresizingMask = [.width]
        view.setAccessibilityLabel(L10n.t("Markdown 邮件正文"))
        view.string = text
        view.delegate = context.coordinator
        controller.textView = view
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let view = scroll.documentView as? NSTextView else { return }
        if view.string != text, !view.hasMarkedText() {
            let range = view.selectedRange()
            view.string = text
            view.setSelectedRange(NSRange(location: min(range.location, (text as NSString).length), length: 0))
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: MarkdownTextView
        init(_ parent: MarkdownTextView) { self.parent = parent }
        func textDidChange(_ notification: Notification) {
            guard let view = notification.object as? NSTextView else { return }
            parent.text = view.string
        }

    }
}
#endif
