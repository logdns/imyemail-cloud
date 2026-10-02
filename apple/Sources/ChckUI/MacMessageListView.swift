#if os(macOS)
import ChckAppCore
import ChckDesign
import SwiftUI

struct MacMessageListView: View {
    @Bindable var store: MailboxStore
    @Environment(\.colorScheme) private var scheme
    private var palette: MailPalette { MailPalette(scheme: scheme) }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(title).font(.system(size: 14, weight: .semibold))
                Text(L10n.t("\(store.visibleMessages.filter(\.unread).count) 封未读"))
                    .font(.system(size: 10)).foregroundStyle(palette.muted)
                Spacer(minLength: 2)
                Menu {
                    Picker(L10n.t("筛选"), selection: $store.filter) {
                        ForEach(MessageFilter.allCases) { Text($0.title).tag($0) }
                    }
                    Divider()
                    Picker(L10n.t("密度"), selection: $store.density) {
                        ForEach(MailDensity.allCases) { Text($0.title).tag($0) }
                    }
                } label: {
                    Image(systemName: store.filter == .all ? "line.3.horizontal.decrease" : "line.3.horizontal.decrease.circle.fill")
                        .font(.system(size: 12)).foregroundStyle(store.filter == .all ? palette.muted : palette.accent)
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel(L10n.t("筛选与列表密度"))
            }
            .padding(.horizontal, 20).frame(height: 56)
            Rectangle().fill(palette.line).frame(height: 1)
            if store.visibleMessages.isEmpty {
                emptyState
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(store.visibleMessages) { row in
                                message(row).id(row.id)
                            }
                        }
                    }
                    .focusable()
                    .onKeyPress(.downArrow) { moveSelection(1); return .handled }
                    .onKeyPress(.upArrow) { moveSelection(-1); return .handled }
                    .onChange(of: store.selectedMessage?.id) { _, id in
                        if let id { proxy.scrollTo(id) }
                    }
                }
            }
            HStack {
                Text(store.searchText.isEmpty ? L10n.t("\(store.visibleMessages.count) 封邮件") : L10n.t("搜索结果 · \(store.visibleMessages.count) 封"))
                Spacer()
                if store.filter != .all {
                    Button("\(store.filter.title) ×") { store.filter = .all }
                        .buttonStyle(.plain).foregroundStyle(palette.accent)
                }
            }
            .font(.system(size: 10)).foregroundStyle(palette.muted)
            .padding(.horizontal, 20).frame(height: 32)
            .overlay(alignment: .top) { Rectangle().fill(palette.line).frame(height: 1) }
        }
        .background(palette.surface)
    }

    private func message(_ row: MailRow) -> some View {
        let selected = store.selectedMessage?.id == row.id
        return Button { Task { await store.open(row) } } label: {
            HStack(alignment: .top, spacing: 11) {
                SenderAvatar(name: row.senderLabel, size: store.density == .compact ? 28 : 34)
                VStack(alignment: .leading, spacing: 5) {
                    HStack(spacing: 5) {
                        Text(row.senderLabel)
                            .font(.system(size: 11, weight: row.unread ? .semibold : .medium))
                            .lineLimit(1)
                        Spacer(minLength: 0)
                        Text(row.dateLabel).font(.system(size: 9)).foregroundStyle(palette.muted)
                    }
                    Text(row.subject.isEmpty ? L10n.t("无主题") : row.subject)
                        .font(.system(size: 12, weight: row.unread ? .semibold : .medium))
                        .lineLimit(store.density == .spacious ? 2 : 1)
                    Text(row.snippet).font(.system(size: 11)).foregroundStyle(palette.muted)
                        .lineLimit(store.density == .spacious ? 2 : 1)
                    if store.density != .compact {
                        HStack(spacing: 5) {
                            Circle().fill(ChckColor.hex(store.color(for: row))).frame(width: 4, height: 4)
                            Text(store.accounts.first(where: { $0.id == row.accountId })?.title ?? L10n.t("邮箱"))
                                .font(.system(size: 9)).foregroundStyle(palette.muted).lineLimit(1)
                            Spacer()
                            if row.hasAttachments { Image(systemName: "paperclip").font(.system(size: 10)).foregroundStyle(palette.muted) }
                            if row.flagged { Image(systemName: "star.fill").font(.system(size: 10)).foregroundStyle(.orange.opacity(0.8)) }
                            if row.unread { Circle().fill(palette.accent).frame(width: 5, height: 5) }
                        }
                        .padding(.top, 3)
                    }
                }
            }
            .padding(.horizontal, 18)
            .padding(.vertical, store.density == .compact ? 12 : store.density == .spacious ? 22 : 18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(selected ? palette.selection : palette.surface)
            .overlay(alignment: .leading) { if selected { Rectangle().fill(palette.accent).frame(width: 2) } }
            .overlay(alignment: .bottom) { Rectangle().fill(palette.line.opacity(0.75)).frame(height: 1) }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(L10n.t("\(row.senderLabel)，\(row.subject)，\(row.unread ? "未读" : "已读")"))
        .accessibilityAddTraits(selected ? .isSelected : [])
        .contextMenu {
            Button(L10n.t("回复")) { store.openCompose(reply: row) }
            Button(L10n.t("回复全部")) { store.openCompose(reply: row, replyAll: true) }
            Button(L10n.t("转发")) { store.openCompose(reply: row, forward: true) }
            Divider()
            Button(row.unread ? L10n.t("标记为已读") : L10n.t("标记为未读")) { store.selectedMessage = row; store.markSeen(row.unread) }
            Button(row.flagged ? L10n.t("取消星标") : L10n.t("星标")) { store.selectedMessage = row; store.toggleStar() }
            Divider()
            Button(L10n.t("归档")) { store.selectedMessage = row; store.archiveSelected() }
            Button(L10n.t("删除"), role: .destructive) { store.selectedMessage = row; store.deleteSelected() }
        }
    }

    private var title: String {
        switch store.selectedItem {
        case .unified: L10n.t("收件箱")
        case .starred: L10n.t("星标邮件")
        case .snooze: L10n.t("稍后提醒")
        case .folder: store.selectedFolder?.title ?? L10n.t("邮件")
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: store.isSyncing ? "arrow.triangle.2.circlepath" : "tray")
                .font(.system(size: 26, weight: .light)).foregroundStyle(palette.muted)
            Text(store.accounts.isEmpty ? L10n.t("连接你的第一个邮箱") : store.isSyncing ? L10n.t("正在收取来信…") : L10n.t("这里暂时没有邮件"))
                .font(.system(size: 13, weight: .medium))
            if !store.status.isEmpty { Text(store.status).font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center) }
            Button(store.accounts.isEmpty ? L10n.t("添加账号") : L10n.t("刷新邮箱")) {
                if store.accounts.isEmpty { store.showAddAccount = true }
                else { Task { await store.refresh() } }
            }.buttonStyle(.borderless).tint(palette.accent)
        }
        .padding(24).frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func moveSelection(_ offset: Int) {
        let rows = store.visibleMessages
        guard !rows.isEmpty else { return }
        let current = rows.firstIndex { $0.id == store.selectedMessage?.id } ?? (offset > 0 ? -1 : rows.count)
        let index = min(max(current + offset, 0), rows.count - 1)
        Task { await store.open(rows[index]) }
    }
}
#endif
