import ChckAppCore
import ChckDesign
import SwiftUI

public struct ComposeWindow: View {
    @Bindable var store: MailboxStore
    var draftID: UUID

    public init(store: MailboxStore, draftID: UUID) {
        self.store = store
        self.draftID = draftID
    }

    public var body: some View {
        if let draft = store.draft(for: draftID) {
            ComposeForm(store: store, draft: draft, showsWindowControls: true)
                .id(draft.id)
        } else {
            ContentUnavailableView(L10n.t("草稿已关闭"), systemImage: "envelope")
                .frame(minWidth: 560, minHeight: 420)
        }
    }
}

struct ComposeForm: View {
    @Bindable var store: MailboxStore
    @Bindable var draft: ComposeDraft
    var showsWindowControls = false
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var scheme
    @State private var sending = false
    @FocusState private var focused: Field?

    private enum Field: Hashable {
        case to, cc, bcc, subject, body
    }

    var body: some View {
        #if os(iOS)
        mobileForm
        #else
        VStack(spacing: 0) {
            header
            Divider()
            MarkdownComposer(text: $draft.body)
                .focused($focused, equals: .body)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            if showsWindowControls {
                Divider()
                footer
            }
        }
        .background(.background)
        .frame(minWidth: 640, minHeight: 520)
        .onAppear {
            focused = draft.to.isEmpty ? .to : .body
        }
        #endif
    }

    #if os(iOS)
    private var mobileForm: some View {
        Form {
            Section(L10n.t("发件账号")) {
                Picker(L10n.t("账户"), selection: $draft.accountId) {
                    ForEach(store.accounts) { account in
                        Text(account.email).tag(account.id)
                    }
                }
            }
            Section {
                mobileField(L10n.t("收件人"), text: $draft.to, field: .to)
                if draft.showCc {
                    mobileField(L10n.t("抄送"), text: $draft.cc, field: .cc)
                    mobileField(L10n.t("密送"), text: $draft.bcc, field: .bcc)
                } else {
                    Button(L10n.t("抄送 / 密送")) { draft.showCc = true }
                }
                mobileField(L10n.t("主题"), text: $draft.subject, field: .subject)
            }
            Section(L10n.t("正文")) {
                IOSMarkdownComposer(text: $draft.body)
            }
        }
        .scrollDismissesKeyboard(.interactively)
        .onAppear { focused = draft.to.isEmpty ? .to : .body }
    }

    private func mobileField(_ title: String, text: Binding<String>, field: Field) -> some View {
        TextField(title, text: text)
            .accessibilityLabel(title)
            .accessibilityIdentifier("compose.\(field)")
            .focused($focused, equals: field)
            .textInputAutocapitalization(field == .subject ? .sentences : .never)
            .autocorrectionDisabled(field != .subject)
            .keyboardType(field == .subject ? .default : .emailAddress)
    }
    #endif

    private var header: some View {
        VStack(alignment: .leading, spacing: 8) {
            if store.accounts.count > 1 {
                Picker(L10n.t("账户"), selection: $draft.accountId) {
                    ForEach(store.accounts) { account in
                        Text(account.email).tag(account.id)
                    }
                }
            }
            field(L10n.t("收件人"), text: $draft.to, field: .to)
            if draft.showCc {
                field(L10n.t("抄送"), text: $draft.cc, field: .cc)
                field(L10n.t("密送"), text: $draft.bcc, field: .bcc)
            }
            field(L10n.t("主题"), text: $draft.subject, field: .subject)
        }
        .padding(.horizontal, 24).padding(.vertical, 18)
        #if os(macOS)
        .background(MailPalette(scheme: scheme).topbar)
        #endif
    }

    private func field(_ title: String, text: Binding<String>, field: Field) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(title)
                .foregroundStyle(.secondary)
                .frame(width: 52, alignment: .trailing)
            TextField("", text: text)
                .accessibilityLabel(title)
                .accessibilityIdentifier("compose.\(field)")
                .font(field == .subject ? .system(size: 17, weight: .semibold) : .system(size: 13))
                .textFieldStyle(.plain)
                .focused($focused, equals: field)
                #if os(iOS)
                .textInputAutocapitalization(field == .subject ? .sentences : .never)
                .autocorrectionDisabled(field != .subject)
                .keyboardType(field == .subject ? .default : .emailAddress)
                #endif
            if field == .to, !draft.showCc {
                Button(L10n.t("抄送 / 密送")) { draft.showCc = true }
                    .buttonStyle(.plain)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 4)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }

    private var footer: some View {
        HStack {
            if showsWindowControls {
                Button(L10n.t("取消")) {
                    store.removeDraft(draft.id)
                    dismiss()
                }
                Button(L10n.t("存草稿")) {
                    Task { await saveDraft() }
                }
                .disabled(sending)
            }
            Spacer()
            if !store.status.isEmpty, sending || store.status.contains(L10n.t("失败")) || store.status.contains(L10n.t("认证")) {
                Text(store.status)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .lineLimit(1)
            }
            Button { Task { await send() } } label: {
                Label(L10n.t("发送邮件"), systemImage: "paperplane.fill")
                    .font(.system(size: 12, weight: .medium)).padding(.horizontal, 8).padding(.vertical, 3)
            }
            .buttonStyle(.borderedProminent)
            .tint(ChckColor.accent)
            .keyboardShortcut(.return, modifiers: .command)
            .disabled(!canSend || sending)
        }
        .padding(.horizontal, 20).padding(.vertical, 14)
    }

    private var canSend: Bool {
        !draft.to.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !draft.accountId.isEmpty
    }

    private func saveDraft() async {
        do {
            draft.lastDraftId = try await store.engine.saveDraft(
                accountId: draft.accountId,
                to: draft.to,
                cc: draft.cc,
                bcc: draft.bcc,
                subject: draft.subject,
                body: draft.body,
                html: draft.outgoingHTML
            )
            store.status = L10n.t("草稿已保存")
        } catch {
            store.status = MailboxStore.displayError(error)
        }
    }

    private func send() async {
        sending = true
        store.composeSending = true
        let ok = await store.send(
            accountId: draft.accountId,
            to: draft.to,
            cc: draft.cc,
            bcc: draft.bcc,
            subject: draft.subject,
            body: draft.body,
            html: draft.outgoingHTML
        )
        store.composeSending = false
        sending = false
        if ok {
            store.removeDraft(draft.id)
            dismiss()
        }
    }
}
