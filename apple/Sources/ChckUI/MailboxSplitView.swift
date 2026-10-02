import ChckAppCore
import ChckDesign
import SwiftUI

public struct MailboxSplitView: View {
    @Bindable var store: MailboxStore
    @Environment(\.openWindow) private var openWindow

    public init(store: MailboxStore) {
        self.store = store
    }

    public var body: some View {
        #if os(macOS)
        MacMailboxView(store: store)
            .preferredColorScheme(scheme)
        #else
        VStack(spacing: 0) {
            NewMailBanner(store: store)
            mailbox
            StatusBarView(store: store)
        }
        #endif
    }

    private var mailbox: some View {
        NavigationSplitView(columnVisibility: $store.columnVisibility) {
            SidebarView(store: store)
                .navigationSplitViewColumnWidth(min: ChckSpacing.navMin, ideal: ChckSpacing.navIdeal, max: ChckSpacing.navMax)
        } content: {
            MessageListView(store: store)
                .toolbar { mailboxToolbar }
                .navigationSplitViewColumnWidth(min: ChckSpacing.listMin, ideal: ChckSpacing.listIdeal, max: ChckSpacing.listMax)
        } detail: {
            ReadingPaneView(store: store)
        }
        .navigationSplitViewStyle(.balanced)
        .searchable(text: $store.searchText, isPresented: $store.focusSearch, prompt: L10n.t("搜索邮件"))
        .onSubmit(of: .search) {
            Task { await store.applySearch() }
        }
        .onChange(of: store.searchText) { _, value in
            if value.isEmpty { Task { await store.refresh() } }
        }
        .sheet(isPresented: $store.showAddAccount) {
            AddAccountWizard(store: store)
        }
        #if os(iOS)
        .sheet(isPresented: $store.showSettings) {
            NavigationStack {
                SettingsView(store: store)
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button(L10n.t("关闭")) { store.showSettings = false }
                        }
                    }
            }
        }
        #endif
        .sheet(item: $store.editingAccount) { account in
            EditAccountSheet(store: store, account: account)
        }
        #if os(iOS)
        .sheet(isPresented: $store.showCompose) {
            ComposeSheet(store: store)
        }
        #endif
        .task {
            if !store.usingPreview && store.notificationsEnabled { MailNotifier.requestPermission() }
            await store.bootstrap()
        }
        .task {
            while !Task.isCancelled {
                do { try await Task.sleep(for: .seconds(20)) } catch { break }
                guard !Task.isCancelled else { break }
                await store.tickBackground()
            }
        }
        .onChange(of: store.pendingComposeID) { _, id in
            guard id != nil else { return }
            presentCompose()
        }
        .preferredColorScheme(scheme)
        .tint(ChckColor.accent)
    }

    @ToolbarContentBuilder
    private var mailboxToolbar: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                store.openCompose()
                presentCompose()
            } label: {
                Label(L10n.t("新邮件"), systemImage: "square.and.pencil")
            }
            .disabled(store.accounts.isEmpty)
            settingsButton
            Menu {
                Button { Task { await store.refresh() } } label: {
                    Label(L10n.t("同步"), systemImage: "arrow.clockwise")
                }
                .disabled(store.isSyncing || store.accounts.isEmpty)
                Button { store.showAddAccount = true } label: {
                    Label(L10n.t("添加账号"), systemImage: "plus")
                }
                Button { store.archiveSelected() } label: { Label(L10n.t("归档"), systemImage: "archivebox") }
                    .disabled(store.selectedMessage == nil)
                Button { store.deleteSelected() } label: { Label(L10n.t("删除"), systemImage: "trash") }
                    .disabled(store.selectedMessage == nil)
            } label: {
                Label(L10n.t("更多操作"), systemImage: "ellipsis.circle")
            }
        }
    }

    private func presentCompose() {
        guard let id = store.pendingComposeID else { return }
        #if os(macOS)
        openWindow(id: "compose", value: id)
        store.pendingComposeID = nil
        #else
        _ = id
        #endif
    }

    @ViewBuilder
    private var settingsButton: some View {
        #if os(macOS)
        SettingsLink {
            Label(L10n.t("设置"), systemImage: "gearshape")
        }
        .help(L10n.t("设置 ⌘,"))
        #else
        Button {
            store.showSettings = true
        } label: {
            Label(L10n.t("设置"), systemImage: "gearshape")
        }
        .help(L10n.t("设置"))
        #endif
    }

    private var scheme: ColorScheme? {
        switch store.appearance {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
}
