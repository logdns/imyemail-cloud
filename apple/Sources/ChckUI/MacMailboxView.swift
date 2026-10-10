#if os(macOS)
import ChckAppCore
import ChckDesign
import SwiftUI

struct MacMailboxView: View {
    @Bindable var store: MailboxStore
    @Environment(\.openWindow) private var openWindow
    @Environment(\.colorScheme) private var scheme
    @FocusState private var searchFocused: Bool
    private var palette: MailPalette { MailPalette(scheme: scheme) }

    var body: some View {
        VStack(spacing: 0) {
            topbar
            Rectangle().fill(palette.line).frame(height: 1)
            NewMailBanner(store: store)
            HSplitView {
                if store.columnVisibility == .all || store.columnVisibility == .automatic {
                    MacSidebarView(store: store)
                        .frame(minWidth: 190, idealWidth: 200, maxWidth: 220)
                }
                if store.columnVisibility != .detailOnly {
                    MacMessageListView(store: store)
                        .frame(minWidth: 300, idealWidth: 320, maxWidth: 400)
                }
                MacReadingPane(store: store)
                    .frame(minWidth: 380, idealWidth: 650, maxWidth: .infinity, maxHeight: .infinity)
                    .layoutPriority(1)
            }
        }
        .background(palette.surface)
        .ignoresSafeArea(edges: .top)
        .tint(palette.accent)
        .sheet(isPresented: $store.showAddAccount) { AddAccountWizard(store: store) }
        .sheet(item: $store.editingAccount) { EditAccountSheet(store: store, account: $0) }
        .task {
            if !store.usingPreview && store.notificationsEnabled { MailNotifier.requestPermission() }
            await store.bootstrap()
        }
        .task {
            while !Task.isCancelled {
                do { try await Task.sleep(for: .seconds(20)) } catch { break }
                await store.tickBackground()
            }
        }
        .onChange(of: store.pendingComposeID) { _, id in
            guard let id else { return }
            openWindow(id: "compose", value: id)
            store.pendingComposeID = nil
        }
        .onChange(of: store.focusSearch) { _, focused in
            if focused { searchFocused = true; store.focusSearch = false }
        }
        .onChange(of: store.searchText) { old, value in
            if value.isEmpty && !old.isEmpty { Task { await store.refresh() } }
        }
        .onChange(of: store.filter) { _, value in store.unreadOnly = value == .unread }
        .onChange(of: store.appearance) { _, _ in if !store.usingPreview { store.persistPrefs() } }
        .onChange(of: store.density) { _, _ in if !store.usingPreview { store.persistPrefs() } }
    }

    private var topbar: some View {
        HStack(spacing: 12) {
            Text(ChckBrand.product)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)
                .padding(.leading, 82)
            Spacer()
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass").font(.system(size: 11)).foregroundStyle(palette.muted)
                TextField(L10n.t("搜索你的邮件"), text: $store.searchText)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12))
                    .focused($searchFocused)
                    .onSubmit { Task { await store.applySearch() } }
                    .accessibilityLabel(L10n.t("搜索邮件"))
                if !store.searchText.isEmpty {
                    Button { store.searchText = "" } label: { Image(systemName: "xmark.circle.fill") }
                        .buttonStyle(.plain).foregroundStyle(palette.muted).help(L10n.t("清除搜索"))
                } else {
                    Text("⌘F").font(.system(size: 10)).foregroundStyle(palette.muted)
                }
            }
            .padding(.horizontal, 10).padding(.vertical, 7)
            .frame(maxWidth: 340)
            .background(palette.sidebar, in: RoundedRectangle(cornerRadius: 7))
            Menu {
                Picker(L10n.t("外观"), selection: $store.appearance) {
                    ForEach(MailAppearance.allCases) { Text($0.title).tag($0) }
                }
                Divider()
                Picker(L10n.t("列表密度"), selection: $store.density) {
                    ForEach(MailDensity.allCases) { Text($0.title).tag($0) }
                }
                Divider()
                Button(L10n.t("三栏布局")) { store.setColumn(1) }
                Button(L10n.t("双栏布局")) { store.setColumn(2) }
                Button(L10n.t("专注阅读")) { store.setColumn(3) }
            } label: {
                Image(systemName: "slider.horizontal.3").frame(width: 26, height: 28)
            }
            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            .help(L10n.t("外观与布局")).accessibilityLabel(L10n.t("外观与布局"))
            MailIconButton(title: L10n.t("同步邮件 ⌘R"), symbol: "arrow.clockwise") { Task { await store.refresh() } }
                .disabled(store.isSyncing || store.accounts.isEmpty)
        }
        .padding(.trailing, 16)
        .frame(height: 48)
        .background(palette.topbar)
    }
}
#endif
