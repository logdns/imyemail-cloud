import ChckDesign
#if os(iOS)
import ChckAppCore
import SwiftUI
import UIKit

struct IOSMarkdownComposer: View {
    @Binding var text: String
    @StateObject private var editor = IOSMarkdownEditorController()
    @State private var preview = false
    @State private var html = ""
    @State private var importing = false

    var body: some View {
        VStack(spacing: 10) {
            Picker(L10n.t("正文显示"), selection: $preview) {
                Text(L10n.t("编辑")).tag(false)
                Text(L10n.t("预览")).tag(true)
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("compose.mode")
            if !preview {
              ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(MarkdownFormatting.allCases) { format in
                        Button { editor.apply(format) } label: {
                            Image(systemName: format.symbol).frame(width: 36, height: 36)
                        }
                        .buttonStyle(.bordered)
                        .accessibilityLabel(format.title)
                        .accessibilityIdentifier("markdown.\(format.rawValue)")
                        .disabled(preview)
                    }
                }
            }
            HStack {
                Button { editor.textView?.undoManager?.undo() } label: {
                    Label(L10n.t("撤销"), systemImage: "arrow.uturn.backward")
                }
                .accessibilityIdentifier("markdown.undo")
                Button { editor.textView?.undoManager?.redo() } label: {
                    Label(L10n.t("重做"), systemImage: "arrow.uturn.forward")
                }
                .accessibilityIdentifier("markdown.redo")
                Spacer()
            }
            .buttonStyle(.borderless)
            .disabled(preview)
            }
            IOSMarkdownTextView(text: $text, controller: editor, onPreview: { preview = true })
                .frame(height: preview ? 0 : 280)
                .clipped()
                .accessibilityHidden(preview)
            if preview {
                MailWebView(html: html, allowRemote: false)
                    .frame(height: 320)
                    .accessibilityLabel(L10n.t("Markdown 正文预览"))
                    .accessibilityIdentifier("compose.preview")
            }
            Button { importing = true } label: {
                Label(L10n.t("从文件导入正文"), systemImage: "doc.badge.plus")
            }
            .buttonStyle(.bordered)
            .accessibilityIdentifier("compose.import")
            Text(L10n.t("Markdown · 发送时保留排版"))
                .font(.caption).foregroundStyle(.secondary)
        }
        .sheet(isPresented: $importing) {
            DocumentImportSheet { source in
                // Append to the latest draft, preserving edits made before conversion finished.
                text += (text.isEmpty || text.hasSuffix("\n\n") ? "" : "\n\n") + source
            }
        }
        .onChange(of: preview) { _, value in
            if value { editor.textView?.resignFirstResponder() }
        }
        .task(id: text) {
            let source = text
            do { try await Task.sleep(for: .milliseconds(150)) } catch { return }
            let rendered = await Task.detached(priority: .userInitiated) { MarkdownBody.render(source) }.value
            guard !Task.isCancelled else { return }
            html = rendered
        }
    }
}

@MainActor
final class IOSMarkdownEditorController: ObservableObject {
    weak var textView: UITextView?

    func apply(_ format: MarkdownFormatting) {
        guard let view = textView, view.markedTextRange == nil else { return }
        let edit = format.edit(view.text, selection: view.selectedRange)
        guard let start = view.position(from: view.beginningOfDocument, offset: edit.range.location),
              let end = view.position(from: start, offset: edit.range.length),
              let range = view.textRange(from: start, to: end) else { return }
        view.becomeFirstResponder()
        view.replace(range, withText: edit.replacement)
        view.selectedRange = edit.selection
        view.delegate?.textViewDidChange?(view)
        view.scrollRangeToVisible(edit.selection)
    }
}

private final class IOSMarkdownNativeTextView: UITextView {
    var format: ((MarkdownFormatting) -> Void)?
    override var keyCommands: [UIKeyCommand]? {
        [UIKeyCommand(input: "b", modifierFlags: .command, action: #selector(bold)),
         UIKeyCommand(input: "i", modifierFlags: .command, action: #selector(italic)),
         UIKeyCommand(input: "k", modifierFlags: .command, action: #selector(link))]
    }
    @objc private func bold() { format?(.bold) }
    @objc private func italic() { format?(.italic) }
    @objc private func link() { format?(.link) }
}

private struct IOSMarkdownTextView: UIViewRepresentable {
    @Binding var text: String
    let controller: IOSMarkdownEditorController
    let onPreview: () -> Void

    func makeUIView(context: Context) -> UITextView {
        let view = IOSMarkdownNativeTextView()
        view.format = { [weak controller] in controller?.apply($0) }
        view.font = .preferredFont(forTextStyle: .body)
        view.adjustsFontForContentSizeCategory = true
        view.backgroundColor = .secondarySystemGroupedBackground
        view.textColor = .label
        view.smartQuotesType = .no
        view.smartDashesType = .no
        view.text = text
        view.accessibilityLabel = L10n.t("Markdown 邮件正文")
        view.accessibilityIdentifier = "compose.body"
        view.delegate = context.coordinator
        controller.textView = view
        let toolbar = UIToolbar()
        let preview = UIBarButtonItem(title: L10n.t("预览正文"), primaryAction: UIAction { [weak coordinator = context.coordinator] _ in
            coordinator?.parent.onPreview()
        })
        preview.accessibilityIdentifier = "keyboard.preview"
        let done = UIBarButtonItem(title: L10n.t("收起键盘"), primaryAction: UIAction { [weak view] _ in
            view?.resignFirstResponder()
        })
        toolbar.items = [preview, .flexibleSpace(), done]
        toolbar.sizeToFit()
        view.inputAccessoryView = toolbar
        return view
    }

    func updateUIView(_ view: UITextView, context: Context) {
        context.coordinator.parent = self
        guard view.markedTextRange == nil, view.text != text else { return }
        let selection = view.selectedRange
        view.text = text
        view.selectedRange = NSRange(location: min(selection.location, (text as NSString).length), length: 0)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: IOSMarkdownTextView
        init(_ parent: IOSMarkdownTextView) { self.parent = parent }
        func textViewDidChange(_ textView: UITextView) { parent.text = textView.text }
    }
}
#endif
