import ChckAppCore
import ChckDesign
import SwiftUI

struct ReadingPaneView: View {
    @Bindable var store: MailboxStore
    var showsBottomBar = false
    @State private var sendingQuickReply = false

    var body: some View {
        Group {
            if store.accounts.isEmpty {
                ContentUnavailableView {
                    Label(L10n.t("添加账号"), systemImage: "envelope.badge.plus")
                } description: {
                    Text(store.engineUnavailable ? L10n.t("邮件服务未能启动，请重新打开或重新安装应用。") : L10n.t("连接你的邮箱，开启一处安静的阅读空间。"))
                } actions: {
                    Button(L10n.t("添加账号")) { store.showAddAccount = true }
                }
            } else if let row = store.selectedMessage, let body = store.body, body.id == row.id {
                VStack(spacing: 0) {
                    header(row)
                    Divider()
                    if body.remoteBlocked > 0 && !allowsRemoteImages {
                        HStack {
                            Label(L10n.t("已阻止 \(body.remoteBlocked) 个远程内容，保护阅读隐私"), systemImage: "hand.raised")
                            Spacer()
                        }
                        .font(.caption)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(.yellow.opacity(0.15))
                    }
                    if body.html.isEmpty {
                        ScrollView {
                            Text(body.text)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding()
                        }
                    } else {
                        MailWebView(html: body.html, allowRemote: allowsRemoteImages)
                    }
                    if !showsBottomBar {
                        Divider()
                        HStack {
                            TextField(L10n.t("快速回复"), text: $store.quickReply, axis: .vertical)
                                .textFieldStyle(.roundedBorder)
                                .lineLimit(1...4)
                            Button(L10n.t("发送")) {
                                guard !sendingQuickReply else { return }
                                let text = store.quickReply
                                sendingQuickReply = true
                                Task {
                                    defer { sendingQuickReply = false }
                                    let sent = await store.send(
                                        accountId: row.accountId.isEmpty ? (store.accounts.first?.id ?? "") : row.accountId,
                                        to: row.replyAddress,
                                        cc: "",
                                        bcc: "",
                                        subject: MailboxStore.replySubject(row.subject, forward: false),
                                        body: text
                                    )
                                    if sent, store.selectedMessage?.id == row.id, store.quickReply == text {
                                        store.quickReply = ""
                                    }
                                }
                            }
                            .disabled(sendingQuickReply || store.quickReply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        }
                        .padding(12)
                    }
                }
                .safeAreaInset(edge: .bottom) {
                    if showsBottomBar {
                        bottomBar(row)
                    }
                }
            } else if store.isLoadingBody, let row = store.selectedMessage {
                VStack(alignment: .leading, spacing: 0) {
                    header(row)
                    Divider()
                    Text(row.snippet)
                        .font(.body).foregroundStyle(.secondary)
                        .padding(24)
                    ProgressView(L10n.t("正在加载正文…")).controlSize(.small).padding(.horizontal, 24)
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else if store.selectedMessage != nil {
                ContentUnavailableView {
                    Label(L10n.t("暂时无法打开"), systemImage: "envelope.badge")
                } description: {
                    Text(store.status)
                } actions: {
                    Button(L10n.t("重试")) {
                        if let row = store.selectedMessage { Task { await store.open(row) } }
                    }
                }
            } else {
                VStack(spacing: 18) {
                    BrandMark(size: 56)
                    Text(L10n.t("留白，也是收获"))
                        .font(.system(.title, design: .rounded, weight: .semibold))
                    Text(L10n.t("选择一封来信，专注此刻。"))
                        .font(.callout).foregroundStyle(.secondary)
                    #if os(macOS)
                    Text(L10n.t("⌘N 写信   ·   ⌘R 收取邮件"))
                        .font(.caption).foregroundStyle(.tertiary).padding(.top, 8)
                    #endif
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(ChckColor.brand.opacity(0.025))
            }
        }
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
    }

    private var allowsRemoteImages: Bool {
        store.loadRemoteImages || store.loadRemoteOnce
    }

    @ViewBuilder
    private func header(_ row: MailRow) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(row.subject.isEmpty ? L10n.t("(无主题)") : row.subject)
                    .font(.system(.title2, design: .rounded, weight: .semibold))
                    .lineLimit(2)
                Spacer(minLength: 8)
                if !showsBottomBar {
                    HStack(spacing: 4) {
                        Button { store.openCompose(reply: row) } label: { Label(L10n.t("回复"), systemImage: "arrowshape.turn.up.left").labelStyle(.iconOnly) }
                        Button { store.openCompose(reply: row, replyAll: true) } label: { Label(L10n.t("回复全部"), systemImage: "arrowshape.turn.up.left.2").labelStyle(.iconOnly) }
                        Button { store.openCompose(reply: row, forward: true) } label: { Label(L10n.t("转发"), systemImage: "arrowshape.turn.up.right").labelStyle(.iconOnly) }
                    }
                    .buttonStyle(.borderless)
                }
            }
            HStack(spacing: 8) {
                Circle()
                    .fill(ChckColor.hex(store.color(for: row)))
                    .frame(width: 26, height: 26)
                    .overlay {
                        Text(String(row.senderLabel.prefix(1)))
                            .foregroundStyle(.white)
                            .font(.caption.weight(.semibold))
                    }
                VStack(alignment: .leading, spacing: 1) {
                    Text(row.senderLabel).font(.subheadline.weight(.medium))
                    Text(row.from).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }
                Spacer()
                Text(row.dateLabel).foregroundStyle(.secondary).font(.caption.monospacedDigit())
            }
            if row.hasAttachments {
                Label(L10n.t("包含附件"), systemImage: "paperclip")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 20)
    }

    private func bottomBar(_ row: MailRow) -> some View {
        HStack(spacing: 20) {
            Button { store.archiveSelected() } label: { Label(L10n.t("归档"), systemImage: "archivebox").labelStyle(.iconOnly) }
            Button { store.deleteSelected() } label: { Label(L10n.t("删除"), systemImage: "trash").labelStyle(.iconOnly) }
            Button { store.status = L10n.t("移动将在 M2 提供") } label: { Image(systemName: "folder") }
            Spacer()
            Button { store.openCompose(reply: row) } label: { Label(L10n.t("回复"), systemImage: "arrowshape.turn.up.left").labelStyle(.iconOnly) }
            Button { store.openCompose() } label: { Label(L10n.t("写信"), systemImage: "square.and.pencil").labelStyle(.iconOnly) }
        }
        .font(.title3)
        .padding(.horizontal, 24)
        .padding(.vertical, 10)
        .background(.bar)
        .accessibilityLabel(L10n.t("阅读操作"))
    }
}
