#if os(macOS)
import ChckAppCore
import ChckDesign
import SwiftUI

struct MacSidebarView: View {
    @Bindable var store: MailboxStore
    @Environment(\.colorScheme) private var scheme
    private var palette: MailPalette { MailPalette(scheme: scheme) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 10) {
                BrandMark(size: 28)
                Text(L10n.t("我的邮箱")).font(.system(size: 13, weight: .semibold))
                Spacer()
                Menu {
                    Button(L10n.t("添加账号…")) { store.showAddAccount = true }
                    Divider()
                    ForEach(store.accounts) { account in
                        Button(account.email) { store.editingAccount = account }
                    }
                } label: { Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold)) }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel(L10n.t("管理邮箱账号"))
            }
            .padding(.horizontal, 18).padding(.top, 22).padding(.bottom, 20)
            Button { store.openCompose() } label: {
                HStack(spacing: 9) {
                    Image(systemName: "plus").font(.system(size: 12, weight: .medium))
                    Text(L10n.t("写一封新邮件")).font(.system(size: 12, weight: .medium))
                    Spacer()
                    Text("⌘N").font(.system(size: 10)).foregroundStyle(palette.muted)
                }
                .padding(.horizontal, 12).frame(height: 36)
                .background(palette.surface, in: RoundedRectangle(cornerRadius: 7))
                .overlay(RoundedRectangle(cornerRadius: 7).stroke(palette.line, lineWidth: 1))
                .shadow(color: .black.opacity(scheme == .dark ? 0 : 0.025), radius: 3, y: 2)
            }
            .buttonStyle(.plain).help(L10n.t("新邮件 ⌘N"))
            .padding(.horizontal, 14).padding(.bottom, 18)

            ScrollView {
                VStack(alignment: .leading, spacing: 3) {
                    navigationRow(L10n.t("全部收件箱"), symbol: "tray.full", item: .unified, count: store.unreadCount)
                    navigationRow(L10n.t("星标邮件"), symbol: "star", item: .starred)
                    if store.accounts.count == 1 {
                        ForEach(store.folders.filter { ["sent", "drafts"].contains($0.role) }) { folder in
                            navigationRow(folder.title, symbol: folder.systemImage, item: .folder(folder.id), count: Int(folder.unread))
                        }
                    }
                    HStack {
                        Text(L10n.t("我的账号")).font(.system(size: 9, weight: .medium)).tracking(1.8)
                        Spacer()
                        Button { store.showAddAccount = true } label: { Image(systemName: "plus").font(.system(size: 10)) }
                            .buttonStyle(.plain).help(L10n.t("添加账号")).accessibilityLabel(L10n.t("添加账号"))
                    }
                    .foregroundStyle(palette.muted)
                    .padding(.horizontal, 12).padding(.top, 28).padding(.bottom, 10)
                    ForEach(store.accounts) { account in
                        DisclosureGroup {
                            VStack(spacing: 2) {
                                ForEach(store.folders(for: account.id)) { folder in
                                    navigationRow(folder.title, symbol: folder.systemImage, item: .folder(folder.id), count: Int(folder.unread))
                                }
                                Button { store.editingAccount = account } label: {
                                    Label(L10n.t("账号设置"), systemImage: "slider.horizontal.3")
                                        .font(.system(size: 11)).foregroundStyle(palette.muted)
                                        .frame(maxWidth: .infinity, alignment: .leading).padding(.leading, 12).padding(.vertical, 8)
                                }.buttonStyle(.plain)
                            }
                        } label: {
                            HStack(spacing: 9) {
                                Circle().fill(ChckColor.hex(account.color)).frame(width: 6, height: 6)
                                Text(account.title).font(.system(size: 12)).lineLimit(1)
                                if account.isAuthRequired { Image(systemName: "exclamationmark.circle.fill").foregroundStyle(ChckColor.danger) }
                            }
                            .padding(.vertical, 7)
                            .help(account.email)
                        }
                        .tint(palette.muted)
                        .padding(.horizontal, 10)
                        .contextMenu { Button(L10n.t("编辑账号")) { store.editingAccount = account } }
                    }
                    if store.accounts.isEmpty {
                        Text(L10n.t("连接邮箱，让来信在这里相遇。"))
                            .font(.system(size: 11)).foregroundStyle(palette.muted)
                            .padding(12)
                    }
                }
                .padding(.horizontal, 10)
            }
            Spacer(minLength: 12)
            VStack(alignment: .leading, spacing: 12) {
                SettingsLink {
                    Label(L10n.t("设置"), systemImage: "gearshape")
                        .font(.system(size: 12)).foregroundStyle(.secondary)
                }
                .buttonStyle(.plain).help(L10n.t("设置 ⌘,"))
                HStack(spacing: 6) {
                    if store.isSyncing {
                        ProgressView().controlSize(.mini)
                    } else {
                        Circle().fill(store.isOffline ? .orange : ChckColor.success.opacity(0.75)).frame(width: 4, height: 4)
                    }
                    Text(statusLabel).font(.system(size: 10)).foregroundStyle(palette.muted).lineLimit(2)
                }
                if store.outboxCount > 0 {
                    Button { Task { await store.retryOutbox() } } label: {
                        Label(L10n.t("发件队列 · \(store.outboxCount)"), systemImage: "paperplane")
                            .font(.system(size: 10)).foregroundStyle(palette.accent)
                    }.buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 22).padding(.bottom, 20)
        }
        .frame(maxHeight: .infinity)
        .background(palette.sidebar)
    }

    private var statusLabel: String {
        if store.usingPreview { return L10n.t("示例邮箱 · 演示模式") }
        if store.isSyncing { return L10n.t("正在更新邮件…") }
        if store.isOffline { return L10n.t("当前离线") }
        if !store.status.isEmpty { return store.status }
        return store.accounts.isEmpty ? L10n.t("还未连接邮箱") : L10n.t("邮件已更新")
    }

    private func navigationRow(_ title: String, symbol: String, item: SidebarItem, count: Int = 0) -> some View {
        let selected = store.selectedItem == item
        return Button { Task { await store.select(item) } } label: {
            HStack(spacing: 10) {
                Image(systemName: symbol).font(.system(size: 12)).frame(width: 15)
                Text(title).font(.system(size: 12, weight: selected ? .medium : .regular)).lineLimit(1)
                Spacer(minLength: 4)
                if count > 0 { Text("\(count)").font(.system(size: 10, weight: .medium).monospacedDigit()) }
            }
            .foregroundStyle(selected ? palette.accent : palette.muted)
            .padding(.horizontal, 11).frame(height: 33)
            .background(selected ? palette.selection : .clear, in: RoundedRectangle(cornerRadius: 6))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
#endif
