import ChckAppCore
import ChckDesign
import SwiftUI

struct SidebarView: View {
    @Bindable var store: MailboxStore
    var usesNavigationLinks = false

    var body: some View {
        List(selection: usesNavigationLinks ? .constant(nil) : selection) {
            Section {
                row(title: L10n.t("统一收件箱"), systemImage: "tray.2", item: .unified)
                row(title: L10n.t("星标"), systemImage: "star", item: .starred)
                row(title: L10n.t("稍后"), systemImage: "clock", item: .snooze)
            }
            ForEach(store.accounts) { account in
                Section {
                    Button {
                        store.editingAccount = account
                    } label: {
                        HStack(spacing: 8) {
                            Circle()
                                .fill(ChckColor.hex(account.color).opacity(0.18))
                                .frame(width: 22, height: 22)
                                .overlay {
                                    Text(account.initial)
                                        .font(.system(size: 10, weight: .semibold))
                                        .foregroundStyle(ChckColor.hex(account.color))
                                }
                            VStack(alignment: .leading, spacing: 0) {
                                Text(account.title)
                                    .lineLimit(1)
                                if account.title != account.email {
                                    Text(account.email)
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                            }
                            Spacer(minLength: 0)
                            if account.isAuthRequired {
                                Circle().fill(ChckColor.danger).frame(width: 7, height: 7)
                            }
                            Image(systemName: "pencil")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .buttonStyle(.plain)
                    .help(L10n.t("编辑 \(account.email)"))
                    .contextMenu {
                        Button(L10n.t("编辑账号")) { store.editingAccount = account }
                        Button(L10n.t("删除账号"), role: .destructive) {
                            Task { await store.removeAccount(account) }
                        }
                    }
                    ForEach(store.folders(for: account.id)) { folder in
                        folderRow(folder)
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .foregroundStyle(.primary)
        .safeAreaInset(edge: .top, spacing: 0) {
            HStack(spacing: 10) {
                BrandMark(size: 34)
                VStack(alignment: .leading, spacing: 2) {
                    Text(ChckBrand.product).font(.system(.headline, design: .rounded))
                    Text(L10n.t("留一点时间，给重要的事"))
                        .font(.caption2).foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 16).padding(.vertical, 18)
            .accessibilityElement(children: .combine)
        }
        .navigationTitle("")
        #if os(macOS)
        .navigationSplitViewColumnWidth(min: ChckSpacing.navMin, ideal: ChckSpacing.navIdeal, max: ChckSpacing.navMax)
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button {
                    store.showAddAccount = true
                } label: {
                    Label(L10n.t("添加账号"), systemImage: "plus")
                }
                .help(L10n.t("添加账号"))
            }
        }
        #endif
        .safeAreaInset(edge: .bottom, spacing: 0) {
            VStack(spacing: 0) {
                Divider()
                settingsButton
                    .buttonStyle(.plain)
                    .background(.bar)
            }
        }
    }

    @ViewBuilder
    private var settingsButton: some View {
        let label = Label(L10n.t("设置"), systemImage: "gearshape")
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
        #if os(macOS)
        SettingsLink { label }
        #else
        Button { store.showSettings = true } label: { label }
        #endif
    }

    @ViewBuilder
    private func row(title: String, systemImage: String, item: SidebarItem) -> some View {
        if usesNavigationLinks {
            NavigationLink(value: MailRoute.mailbox(item)) {
                Label(title, systemImage: systemImage)
            }
        } else {
            Label(title, systemImage: systemImage)
                .tag(item)
        }
    }

    @ViewBuilder
    private func folderRow(_ folder: MailFolder) -> some View {
        let label = HStack {
            Label(folder.title, systemImage: folder.systemImage)
            Spacer()
            if folder.unread > 0 {
                Text("\(folder.unread)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(ChckColor.brand)
            }
        }
        if usesNavigationLinks {
            NavigationLink(value: MailRoute.mailbox(.folder(folder.id))) { label }
        } else {
            label.tag(SidebarItem.folder(folder.id))
        }
    }

    private var selection: Binding<SidebarItem?> {
        Binding(
            get: { store.selectedItem },
            set: { item in
                guard let item else { return }
                Task { await store.select(item) }
            }
        )
    }
}
