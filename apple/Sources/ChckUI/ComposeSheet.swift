import ChckDesign
import ChckAppCore
import SwiftUI

public struct ComposeSheet: View {
    @Bindable var store: MailboxStore
    @Environment(\.dismiss) private var dismiss
    @State private var busy = false
    @State private var confirmClose = false
    @State private var errorMessage: String?

    public init(store: MailboxStore) {
        self.store = store
    }

    public var body: some View {
        NavigationStack {
            if let draft = activeDraft {
                ComposeForm(store: store, draft: draft, showsWindowControls: false)
                    .disabled(busy)
                    .navigationTitle(draft.subject.isEmpty ? L10n.t("写信") : draft.subject)
                    #if os(iOS)
                    .navigationBarTitleDisplayMode(.inline)
                    #endif
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button(L10n.t("取消")) { confirmClose = true }
                                .disabled(busy)
                        }
                        ToolbarItem(placement: .confirmationAction) {
                            Button(busy ? L10n.t("处理中…") : L10n.t("发送")) {
                                guard !busy else { return }
                                busy = true
                                Task {
                                    defer { busy = false }
                                    let ok = await store.send(
                                        accountId: draft.accountId, to: draft.to,
                                        cc: draft.cc, bcc: draft.bcc,
                                        subject: draft.subject, body: draft.body,
                                        html: draft.outgoingHTML
                                    )
                                    if ok { close(draft) }
                                    else { errorMessage = store.status }
                                }
                            }
                            .accessibilityIdentifier("compose.send")
                            .disabled(busy || draft.accountId.isEmpty || draft.to.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        }
                    }
                    .confirmationDialog(L10n.t("保存这封邮件？"), isPresented: $confirmClose, titleVisibility: .visible) {
                        Button(L10n.t("保存草稿")) {
                            guard !busy else { return }
                            busy = true
                            Task {
                                defer { busy = false }
                                do {
                                    draft.lastDraftId = try await store.engine.saveDraft(
                                        accountId: draft.accountId, to: draft.to,
                                        cc: draft.cc, bcc: draft.bcc,
                                        subject: draft.subject, body: draft.body,
                                        html: draft.outgoingHTML
                                    )
                                    store.status = L10n.t("草稿已保存")
                                    close(draft)
                                } catch {
                                    errorMessage = MailboxStore.displayError(error)
                                }
                            }
                        }
                        Button(L10n.t("删除草稿"), role: .destructive) { close(draft) }
                        Button(L10n.t("继续编辑")) {}
                    }
            } else {
                ContentUnavailableView(L10n.t("草稿已关闭"), systemImage: "envelope")
            }
        }
        .interactiveDismissDisabled()
        .alert(L10n.t("邮件未完成"), isPresented: Binding(
            get: { errorMessage != nil },
            set: { if !$0 { errorMessage = nil } }
        )) {
            Button(L10n.t("好")) { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "")
        }
        #if os(iOS)
        .presentationDetents([.large])
        #endif
    }

    private var activeDraft: ComposeDraft? {
        guard let id = store.pendingComposeID else { return nil }
        return store.draft(for: id)
    }

    private func close(_ draft: ComposeDraft) {
        store.removeDraft(draft.id)
        store.showCompose = false
        dismiss()
    }
}
