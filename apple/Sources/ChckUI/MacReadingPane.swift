#if os(macOS)
import ChckAppCore
import ChckDesign
import SwiftUI

struct MacReadingPane: View {
    @Bindable var store: MailboxStore
    @Environment(\.colorScheme) private var scheme
    @State private var sending = false
    @State private var replyError: String?
    private var palette: MailPalette { MailPalette(scheme: scheme) }

    var body: some View {
        VStack(spacing: 0) {
            actions
            if let row = store.selectedMessage {
                VStack(alignment: .leading, spacing: 22) {
                    Text(L10n.t("\(store.accounts.first(where: { $0.id == row.accountId })?.title ?? "收件箱")  /  来信"))
                        .font(.system(size: 9, weight: .medium)).tracking(1.5).foregroundStyle(palette.muted)
                    Text(row.subject.isEmpty ? L10n.t("无主题") : row.subject)
                        .font(.system(size: 30, weight: .semibold)).tracking(-0.8)
                        .lineLimit(3).textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .accessibilityAddTraits(.isHeader)
                    HStack(spacing: 11) {
                        SenderAvatar(name: row.senderLabel, size: 34)
                        VStack(alignment: .leading, spacing: 4) {
                            Text(row.senderLabel).font(.system(size: 12, weight: .semibold))
                            Text(row.from).font(.system(size: 10)).foregroundStyle(palette.muted).textSelection(.enabled).lineLimit(1)
                        }
                        Spacer(minLength: 8)
                        Text(row.dateLabel).font(.system(size: 10)).foregroundStyle(palette.muted)
                        MailIconButton(title: L10n.t("回复"), symbol: "arrowshape.turn.up.left") { store.openCompose(reply: row) }
                    }
                }
                .padding(.horizontal, 32).padding(.top, 16).padding(.bottom, 16)
                if let body = store.body, body.id == row.id {
                    if body.remoteBlocked > 0 && !allowsRemoteImages {
                        Label(L10n.t("已阻止远程内容，保护阅读隐私"), systemImage: "lock.shield")
                            .font(.system(size: 10)).foregroundStyle(palette.muted)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, 32).padding(.bottom, 8)
                    }
                    if body.html.isEmpty {
                        ScrollView {
                            Text(body.text).font(.system(size: 14)).lineSpacing(8)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, 32).padding(.vertical, 14)
                        }
                    } else {
                        MailWebView(html: body.html, allowRemote: allowsRemoteImages)
                    }
                    replyBox(row)
                } else if store.isLoadingBody {
                    VStack(alignment: .leading, spacing: 18) {
                        Text(row.snippet).font(.system(size: 13)).foregroundStyle(palette.muted)
                        ProgressView(L10n.t("正在打开来信…")).controlSize(.small)
                        Spacer()
                    }
                    .padding(32).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                } else {
                    ContentUnavailableView {
                        Label(L10n.t("暂时无法打开来信"), systemImage: "envelope.badge")
                    } description: { Text(store.status) } actions: {
                        Button(L10n.t("重新加载")) { Task { await store.open(row) } }
                    }
                }
            } else {
                VStack(spacing: 18) {
                    ZStack {
                        Circle().fill(palette.selection).frame(width: 76, height: 76)
                        Image(systemName: "checkmark").font(.system(size: 28, weight: .light)).foregroundStyle(palette.accent)
                    }
                    Text(store.accounts.isEmpty ? L10n.t("你的邮件，一处安放。") : L10n.t("收件有序，心有余白。"))
                        .font(.system(size: 23, weight: .medium)).tracking(-0.5)
                    Text(store.accounts.isEmpty ? L10n.t("连接常用邮箱，从一封来信开始。") : L10n.t("选择一封来信，留一点时间给重要的事。"))
                        .font(.system(size: 12)).foregroundStyle(palette.muted)
                    if store.accounts.isEmpty {
                        Button(L10n.t("添加邮箱")) { store.showAddAccount = true }.buttonStyle(.borderedProminent)
                    } else {
                        Text(L10n.t("⌘N 写信    ·    ⌘F 搜索"))
                            .font(.system(size: 10)).foregroundStyle(palette.muted.opacity(0.8)).padding(.top, 8)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(maxWidth: 824)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(palette.surface)
        .onChange(of: store.selectedMessage?.id) { _, _ in replyError = nil }
    }

    private var allowsRemoteImages: Bool {
        store.loadRemoteImages || store.loadRemoteOnce
    }

    private var actions: some View {
        HStack(spacing: 6) {
            MailIconButton(title: L10n.t("归档 ⌘E"), symbol: "archivebox") { store.archiveSelected() }
            MailIconButton(title: L10n.t("删除"), symbol: "trash") { store.deleteSelected() }
            MailIconButton(title: L10n.t("标记为未读"), symbol: "envelope.badge") { store.markSeen(false) }
            Spacer()
            MailIconButton(title: store.selectedMessage?.flagged == true ? L10n.t("取消星标") : L10n.t("添加星标"), symbol: store.selectedMessage?.flagged == true ? "star.fill" : "star") { store.toggleStar() }
            Menu {
                if let row = store.selectedMessage {
                    Button(L10n.t("回复")) { store.openCompose(reply: row) }
                    Button(L10n.t("回复全部")) { store.openCompose(reply: row, replyAll: true) }
                    Button(L10n.t("转发")) { store.openCompose(reply: row, forward: true) }
                    Divider()
                    Button(L10n.t("标记为已读")) { store.markSeen(true) }
                    Button(L10n.t("标记为未读")) { store.markSeen(false) }
                }
            } label: { Image(systemName: "ellipsis").frame(width: 28, height: 30) }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .foregroundStyle(palette.muted).accessibilityLabel(L10n.t("更多邮件操作"))
        }
        .disabled(store.selectedMessage == nil)
        .padding(.horizontal, 22).frame(height: 56)
    }

    private func replyBox(_ row: MailRow) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "arrowshape.turn.up.left").font(.system(size: 12)).foregroundStyle(palette.muted).padding(.top, 3)
                TextField(L10n.t("写下你的回复…"), text: $store.quickReply, axis: .vertical)
                    .textFieldStyle(.plain).font(.system(size: 12)).lineLimit(1...5)
                    .disabled(sending)
                Button { sendReply(row) } label: {
                    Image(systemName: sending ? "hourglass" : "arrow.up")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(store.quickReply.isEmpty ? palette.muted : .white)
                        .frame(width: 25, height: 25)
                        .background(store.quickReply.isEmpty ? palette.sidebar : palette.accent, in: RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain).help(L10n.t("发送回复")).accessibilityLabel(L10n.t("发送回复"))
                .disabled(sending || store.quickReply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            .padding(12)
            .background(palette.surface, in: RoundedRectangle(cornerRadius: 9))
            .overlay(RoundedRectangle(cornerRadius: 9).stroke(palette.line, lineWidth: 1))
            if let replyError { Text(replyError).font(.caption).foregroundStyle(.red) }
        }
        .padding(.horizontal, 32).padding(.top, 12).padding(.bottom, 24)
    }

    private func sendReply(_ row: MailRow) {
        guard !sending else { return }
        let text = store.quickReply
        sending = true
        replyError = nil
        Task {
            defer { sending = false }
            let sent = await store.send(accountId: row.accountId.isEmpty ? (store.accounts.first?.id ?? "") : row.accountId,
                                        to: row.replyAddress, cc: "", bcc: "", subject: MailboxStore.replySubject(row.subject, forward: false), body: text)
            if store.selectedMessage?.id == row.id {
                if sent, store.quickReply == text { store.quickReply = "" }
                if !sent { replyError = store.status }
            }
        }
    }
}
#endif
