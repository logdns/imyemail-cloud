import ChckAppCore
import ChckDesign
import SwiftUI

struct MessageListView: View {
    @Bindable var store: MailboxStore
    var embedsNavigation = false

    var body: some View {
        List(selection: embedsNavigation ? .constant(nil) : messageBinding) {
            ForEach(store.visibleMessages) { row in
                messageRow(row)
            }
        }
        .listStyle(.plain)
        #if os(macOS)
        .safeAreaInset(edge: .top, spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(listTitle).font(.system(.title2, design: .rounded, weight: .bold))
                    Text(store.searchText.isEmpty ? L10n.t("每一封来信，都有自己的位置") : L10n.t("搜索结果"))
                        .font(.caption).foregroundStyle(.secondary)
                }
                Spacer(minLength: 4)
                Text("\(store.visibleMessages.count)")
                    .font(.system(.title3, design: .rounded, weight: .medium).monospacedDigit())
                    .foregroundStyle(.secondary)
                    .accessibilityLabel(L10n.t("\(store.visibleMessages.count) 封邮件"))
            }
            .padding(16)
            .background(.bar)
        }
        #endif
        .navigationTitle(listTitle)
        #if os(macOS)
        .navigationSplitViewColumnWidth(min: ChckSpacing.listMin, ideal: ChckSpacing.listIdeal, max: ChckSpacing.listMax)
        #else
        .safeAreaInset(edge: .top, spacing: 0) {
            filterBar
        }
        #endif
        .overlay {
            if store.accounts.isEmpty {
                ContentUnavailableView {
                    VStack(spacing: 12) {
                        BrandMark(size: 72)
                        Text(store.engineUnavailable ? L10n.t("暂时无法启动") : L10n.t("添加账号"))
                            .font(.title2.weight(.semibold))
                    }
                } description: {
                    Text(store.engineUnavailable ? L10n.t("邮件服务未能启动，请重新打开或重新安装应用。") : L10n.t("连接你的邮箱，让重要来信在这里相遇。"))
                } actions: {
                    Button(L10n.t("添加账号")) { store.showAddAccount = true }
                }
            } else if store.isSyncing && store.visibleMessages.isEmpty && store.lastSyncedAt == nil {
                ProgressView(L10n.t("正在同步…"))
            } else if store.visibleMessages.isEmpty {
                ContentUnavailableView {
                    Label(emptyTitle, systemImage: emptyIsError ? "exclamationmark.triangle" : "checkmark.circle")
                } description: {
                    Text(emptyIsError ? store.status : L10n.t("没有邮件"))
                } actions: {
                    Button(L10n.t("同步")) { Task { await store.refresh() } }
                }
            }
        }
        .modifier(PhoneSearchModifier(enabled: embedsNavigation, text: $store.searchText) {
            Task { await store.applySearch() }
        })
    }

    private var filterBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(MessageFilter.allCases) { item in
                    Button {
                        store.filter = item
                        store.unreadOnly = item == .unread
                    } label: {
                        Text(item.title)
                            .font(.caption)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                            .background(store.filter == item ? ChckColor.brand.opacity(0.15) : Color.secondary.opacity(0.08), in: Capsule())
                    }
                    .buttonStyle(.plain)
                }
                if store.isSyncing {
                    ProgressView().controlSize(.small)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
        }
    }

    @ViewBuilder
    private func messageRow(_ row: MailRow) -> some View {
        let content = MessageRowView(row: row, color: store.color(for: row), density: store.density)
            .listRowInsets(EdgeInsets(top: store.density.rowPadding, leading: 14, bottom: store.density.rowPadding, trailing: 14))
            .contextMenu { contextMenu(row) }
            .swipeActions(edge: .leading) {
                Button {
                    store.selectedMessage = row
                    store.markSeen(row.unread)
                } label: {
                    Label(row.unread ? L10n.t("已读") : L10n.t("未读"), systemImage: "envelope.open")
                }
                .tint(ChckColor.accent)
                Button {
                    store.selectedMessage = row
                    store.toggleStar()
                } label: {
                    Label(L10n.t("星标"), systemImage: "star")
                }
                .tint(.yellow)
            }
            .swipeActions(edge: .trailing) {
                Button {
                    store.selectedMessage = row
                    store.archiveSelected()
                } label: {
                    Label(L10n.t("归档"), systemImage: "archivebox")
                }
                .tint(ChckColor.success)
                Button(role: .destructive) {
                    store.selectedMessage = row
                    store.deleteSelected()
                } label: {
                    Label(L10n.t("删除"), systemImage: "trash")
                }
            }

        if embedsNavigation {
            NavigationLink(value: MailRoute.message(row.id)) { content }
        } else {
            content.tag(row.id)
        }
    }

    @ViewBuilder
    private func contextMenu(_ row: MailRow) -> some View {
        Button(L10n.t("回复")) { store.openCompose(reply: row) }
        Button(L10n.t("转发")) { store.openCompose(reply: row, forward: true) }
        Divider()
        Button(row.unread ? L10n.t("标记为已读") : L10n.t("标记为未读")) {
            store.selectedMessage = row
            store.markSeen(row.unread)
        }
        Button(row.flagged ? L10n.t("取消星标") : L10n.t("星标")) {
            store.selectedMessage = row
            store.toggleStar()
        }
        Divider()
        Button(L10n.t("归档")) {
            store.selectedMessage = row
            store.archiveSelected()
        }
        Button(L10n.t("删除"), role: .destructive) {
            store.selectedMessage = row
            store.deleteSelected()
        }
    }

    private var listTitle: String {
        switch store.selectedItem {
        case .unified: L10n.t("统一收件箱")
        case .starred: L10n.t("星标")
        case .snooze: L10n.t("稍后")
        case .folder: store.selectedFolder?.title ?? L10n.t("邮件")
        }
    }

    private var emptyIsError: Bool {
        let text = store.status
        return !text.isEmpty
            && text != L10n.t("已同步")
            && !text.hasPrefix(L10n.t("已同步"))
            && text != L10n.t("已加入发件队列")
            && text != L10n.t("已撤销发送")
            && text != L10n.t("已归档")
            && text != L10n.t("已删除")
    }

    private var emptyTitle: String {
        emptyIsError ? L10n.t("同步未完成") : L10n.t("全部处理完了")
    }

    private var messageBinding: Binding<MailRow.ID?> {
        Binding(
            get: { store.selectedMessage?.id },
            set: { id in
                if let row = store.messages.first(where: { $0.id == id }) {
                    Task { await store.open(row) }
                }
            }
        )
    }
}

private struct PhoneSearchModifier: ViewModifier {
    var enabled: Bool
    @Binding var text: String
    var onSubmit: () -> Void

    func body(content: Content) -> some View {
        if enabled {
            content
                .searchable(text: $text, prompt: L10n.t("搜索邮件"))
                .onSubmit(of: .search, onSubmit)
        } else {
            content
        }
    }
}

struct MessageRowView: View {
    let row: MailRow
    let color: String
    let density: MailDensity

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Circle()
                .fill(row.unread ? ChckColor.unread : .clear)
                .frame(width: 7, height: 7)
                .padding(.top, 6)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(row.senderLabel)
                        .font(.subheadline.weight(row.unread ? .semibold : .regular))
                        .lineLimit(1)
                    Spacer(minLength: 4)
                    if row.flagged {
                        Image(systemName: "star.fill")
                            .font(.caption2)
                            .foregroundStyle(.yellow)
                    }
                    if row.hasAttachments {
                        Image(systemName: "paperclip")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Text(row.dateLabel)
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
                Text(row.subject.isEmpty ? L10n.t("(无主题)") : row.subject)
                    .font(.subheadline.weight(row.unread ? .medium : .regular))
                    .lineLimit(1)
                if density.snippetLines > 0 {
                    Text(row.snippet)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(density.snippetLines)
                }
            }
        }
        .overlay(alignment: .leading) {
            Capsule()
                .fill(ChckColor.hex(color).opacity(row.unread ? 1 : 0.35))
                .frame(width: ChckColor.stripeWidth)
                .offset(x: -8)
        }
    }
}
