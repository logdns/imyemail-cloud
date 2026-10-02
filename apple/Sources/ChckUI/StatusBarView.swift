import ChckAppCore
import ChckDesign
import SwiftUI

struct StatusBarView: View {
    @Bindable var store: MailboxStore

    var body: some View {
        HStack(spacing: 12) {
            if store.usingPreview {
                Text(L10n.t("演示模式 · 示例邮件"))
                    .foregroundStyle(.orange)
            } else if store.isSyncing, store.messages.isEmpty {
                ProgressView().controlSize(.mini)
                Text(L10n.t("同步中"))
            } else if store.isSyncing {
                ProgressView().controlSize(.mini)
                Text(L10n.t("后台更新"))
                    .foregroundStyle(.secondary)
            } else if store.isOffline {
                Image(systemName: "wifi.slash")
                Text(L10n.t("离线中 · 操作已排队"))
            } else if !store.status.isEmpty {
                Text(store.status)
            } else if let at = store.lastSyncedAt {
                Text(L10n.t("已更新 \(at.formatted(date: .omitted, time: .shortened))"))
                    .foregroundStyle(.secondary)
            } else {
                Text(L10n.t("已同步"))
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Label(L10n.t("\(store.unreadCount) 封未读"), systemImage: "envelope.badge")
                .foregroundStyle(.secondary)
            if store.outboxCount > 0 {
                Button {
                    Task { await store.retryOutbox() }
                } label: {
                    Label(L10n.t("发件队列 \(store.outboxCount)"), systemImage: "paperplane")
                }
                .buttonStyle(.plain)
                .help(L10n.t("立即发送队列中的邮件"))
            }
            Text(store.accounts.map(\.email).joined(separator: " · "))
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
