import ChckAppCore
import ChckDesign
import SwiftUI

struct PhoneMailboxView: View {
    @Bindable var store: MailboxStore

    var body: some View {
        NavigationStack(path: $store.phonePath) {
            SidebarView(store: store, usesNavigationLinks: true)
                .navigationTitle(ChckBrand.product)
                #if os(iOS)
                .navigationBarTitleDisplayMode(.large)
                #endif
                .toolbar { toolbar }
                .navigationDestination(for: MailRoute.self, destination: destination)
                .overlay {
                    if store.accounts.isEmpty {
                        ContentUnavailableView {
                            VStack(spacing: 12) {
                                BrandMark(size: 88)
                                Text(ChckBrand.product)
                                    .font(.title2.weight(.semibold))
                            }
                        } description: {
                            Text(store.engineUnavailable ? L10n.t("邮件服务未能启动，请重新打开或重新安装应用。") : ChckBrand.tagline)
                        } actions: {
                            if !store.engineUnavailable {
                                Button(L10n.t("添加账号")) { store.showAddAccount = true }
                            }
                        }
                    }
                }
        }
        .safeAreaInset(edge: .top, spacing: 0) {
            VStack(spacing: 0) {
                if store.usingPreview {
                    Text(L10n.t("演示模式 · 示例邮件"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity)
                        .padding(4)
                }
                NewMailBanner(store: store)
            }
        }
        .sheet(isPresented: $store.showAddAccount) {
            AddAccountWizard(store: store)
        }
        .sheet(isPresented: $store.showCompose) {
            ComposeSheet(store: store)
        }
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
        .sheet(item: $store.editingAccount) { account in
            EditAccountSheet(store: store, account: account)
        }
        .overlay(alignment: .bottomTrailing) {
            if store.phonePath.count < 2 && !store.accounts.isEmpty {
                composeButton
                    .padding(20)
            }
        }
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
    }

    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        #if os(iOS)
        ToolbarItem(placement: .topBarLeading) {
            Button {
                store.showSettings = true
            } label: {
                Label(L10n.t("设置"), systemImage: "gear")
            }
        }
        #else
        ToolbarItem(placement: .automatic) {
            Button {
                store.showSettings = true
            } label: {
                Label(L10n.t("设置"), systemImage: "gear")
            }
        }
        #endif
        ToolbarItem(placement: .primaryAction) {
            Button {
                Task { await store.refresh() }
            } label: {
                Label(L10n.t("同步"), systemImage: "arrow.clockwise")
            }
            .disabled(store.isSyncing || store.accounts.isEmpty)
        }
        ToolbarItem(placement: .primaryAction) {
            Button {
                store.showAddAccount = true
            } label: {
                Label(L10n.t("添加账号"), systemImage: "plus")
            }
        }
    }

    @ViewBuilder
    private func destination(for route: MailRoute) -> some View {
        switch route {
        case .mailbox(let item):
            MessageListView(store: store, embedsNavigation: true)
                .task { await store.select(item) }
        case .message(let id):
            ReadingPaneView(store: store, showsBottomBar: true)
                .task {
                    if let row = store.messages.first(where: { $0.id == id }) {
                        await store.open(row)
                    }
                }
        }
    }

    private var composeButton: some View {
        Button {
            store.openCompose()
        } label: {
            Image(systemName: "square.and.pencil")
                .font(.title3.weight(.semibold))
                .foregroundStyle(.white)
                .frame(width: 56, height: 56)
                .background(ChckColor.brand, in: Circle())
                .shadow(radius: 4, y: 2)
        }
        .contextMenu {
            ForEach(store.accounts) { account in
                Button(account.email) {
                    store.openCompose()
                    if let id = store.pendingComposeID, let draft = store.draft(for: id) {
                        draft.accountId = account.id
                    }
                }
            }
        }
        .accessibilityLabel(L10n.t("写信"))
    }
}
