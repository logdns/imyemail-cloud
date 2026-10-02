import ChckAppCore
import ChckDesign
import SwiftUI

struct MailCommands: Commands {
    @Bindable var store: MailboxStore

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button(L10n.t("新邮件")) { store.openCompose() }
                .keyboardShortcut("n")
            Button(L10n.t("添加账号…")) { store.showAddAccount = true }
                .keyboardShortcut("n", modifiers: [.command, .shift])
        }
        CommandGroup(after: .textEditing) {
            Button(L10n.t("查找邮件")) { store.focusSearch = true }
                .keyboardShortcut("f")
        }
        CommandMenu(L10n.t("邮件")) {
            Button(L10n.t("回复")) { store.openCompose(reply: store.selectedMessage) }
                .disabled(store.selectedMessage == nil)
            Button(L10n.t("回复全部")) { store.openCompose(reply: store.selectedMessage, replyAll: true) }
                .disabled(store.selectedMessage == nil)
            Button(L10n.t("转发")) { store.openCompose(reply: store.selectedMessage, forward: true) }
                .disabled(store.selectedMessage == nil)
            Divider()
            Button(L10n.t("归档")) { store.archiveSelected() }
                .keyboardShortcut("e")
                .disabled(store.selectedMessage == nil)
            Button(L10n.t("删除")) { store.deleteSelected() }
                .disabled(store.selectedMessage == nil)
            Button(L10n.t("星标")) { store.toggleStar() }
                .disabled(store.selectedMessage == nil)
            Divider()
            Button(L10n.t("标记为已读")) { store.markSeen(true) }
                .disabled(store.selectedMessage == nil)
            Button(L10n.t("标记为未读")) { store.markSeen(false) }
                .disabled(store.selectedMessage == nil)
            Divider()
            Button(L10n.t("撤销发送")) { Task { await store.undoLastSend() } }
                .keyboardShortcut("z", modifiers: [.command, .option])
                .disabled(store.lastSentDraftId == nil)
        }
        CommandMenu(L10n.t("邮箱")) {
            Button(L10n.t("同步")) { Task { await store.refresh() } }
                .keyboardShortcut("r")
            Button(L10n.t("统一收件箱")) { Task { await store.select(.unified) } }
            Divider()
            Button(L10n.t("显示三栏")) { store.setColumn(1) }
                .keyboardShortcut("1")
            Button(L10n.t("显示双栏")) { store.setColumn(2) }
                .keyboardShortcut("2")
            Button(L10n.t("只显示阅读")) { store.setColumn(3) }
                .keyboardShortcut("3")
        }
        CommandGroup(replacing: .help) {
            Link(L10n.t("imy.email 官网"), destination: ChckBrand.site)
            Link(L10n.t("隐私政策"), destination: ChckBrand.privacy)
        }
    }
}
