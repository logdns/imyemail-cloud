import ChckAppCore
import ChckDesign
import SwiftUI

public struct SettingsView: View {
    @Bindable var store: MailboxStore
    @AppStorage("chck.language") private var language = "en"
    @State private var pane: SettingsPane? = .appearance
    @State private var editing: MailAccount?
    @State private var confirmClear = false
    @State private var notificationPermission: MailNotificationAuthorizationStatus = .unavailable

    public init(store: MailboxStore) {
        self.store = store
    }

    public var body: some View {
        #if os(macOS)
        NavigationSplitView {
            List(SettingsPane.allCases, id: \.self, selection: $pane) { item in
                Label(item.title, systemImage: item.systemImage)
                    .tag(Optional(item))
            }
            .listStyle(.sidebar)
            .navigationTitle(L10n.t("设置"))
            .navigationSplitViewColumnWidth(min: 148, ideal: 168, max: 200)
        } detail: {
            detail
                .formStyle(.grouped)
                .navigationTitle((pane ?? .accounts).title)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(minWidth: 640, minHeight: 440)
        .sheet(item: $editing, content: editSheet)
        .onChange(of: store.density) { _, _ in store.persistPrefs() }
        .onChange(of: store.appearance) { _, _ in store.persistPrefs() }
        .onChange(of: store.loadRemoteImages) { _, _ in store.persistPrefs() }
        .onChange(of: store.notificationsEnabled) { _, enabled in
            store.persistPrefs()
            Task {
                notificationPermission = enabled ? await MailNotifier.requestAuthorization() : await MailNotifier.authorizationStatus()
                await MailNotifier.setUnreadCount(store.unreadCount)
            }
        }
        #else
        Form {
            appearanceForm
            mailForm
            Section(L10n.t("正文导入")) {
                NavigationLink(L10n.t("文件转换服务")) { DocumentConversionSettings() }
            }
            accountsForm
            aboutForm
        }
        .formStyle(.grouped)
        .navigationTitle(L10n.t("设置"))
        .sheet(item: $editing, content: editSheet)
        .onChange(of: store.density) { _, _ in store.persistPrefs() }
        .onChange(of: store.appearance) { _, _ in store.persistPrefs() }
        .onChange(of: store.loadRemoteImages) { _, _ in store.persistPrefs() }
        .onChange(of: store.notificationsEnabled) { _, enabled in
            store.persistPrefs()
            Task {
                notificationPermission = enabled ? await MailNotifier.requestAuthorization() : await MailNotifier.authorizationStatus()
                await MailNotifier.setUnreadCount(store.unreadCount)
            }
        }
        #endif
    }

    @ViewBuilder
    private var detail: some View {
        switch pane ?? .accounts {
        case .accounts: Form { accountsForm }
        case .appearance: Form { appearanceForm }
        case .mail: Form { mailForm }
        case .about: Form { aboutForm }
        }
    }

    private var appearanceForm: some View {
        Section(L10n.t("外观")) {
            Picker(L10n.t("语言"), selection: $language) {
                ForEach(Array(zip(L10n.codes, L10n.names)), id: \.0) { code, name in
                    Text(verbatim: name).tag(code)
                }
            }
            Text(L10n.t("更改语言将在下次启动时生效。")).font(.caption).foregroundStyle(.secondary)
            #if os(macOS)
            VStack(alignment: .leading, spacing: 14) {
                Text(L10n.t("让邮箱适应你的习惯")).font(.system(size: 18, weight: .semibold))
                Text(L10n.t("浅色清爽，深色沉静；也可以随系统自动切换。"))
                    .font(.callout).foregroundStyle(.secondary)
                AppearanceChoices(store: store)
            }
            .padding(.vertical, 8)
            #else
            Picker(L10n.t("外观"), selection: $store.appearance) {
                ForEach(MailAppearance.allCases) { item in Text(item.title).tag(item) }
            }
            #endif
            Picker(L10n.t("密度"), selection: $store.density) {
                ForEach(MailDensity.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
        }
    }

    private var mailForm: some View {
        Section(L10n.t("邮件")) {
            Toggle(L10n.t("远程图片"), isOn: $store.loadRemoteImages)
            Toggle(L10n.t("新邮件通知"), isOn: $store.notificationsEnabled)
            if notificationPermission == .denied && store.notificationsEnabled {
                Text(L10n.t("系统通知未允许，请在系统设置的通知中开启 imyemail-cloud。应用内仍会提示新邮件。"))
                    .font(.caption).foregroundStyle(.secondary)
            }
            Button(L10n.t("清理失败队列")) { Task { await store.clearFailedQueue() } }
            Text(L10n.t("只清理失败任务，未发送内容保留为草稿。"))
                .font(.caption).foregroundStyle(.secondary)
            LabeledContent(L10n.t("撤销发送窗口"), value: L10n.t("10 秒"))
            Text(L10n.t("远程内容默认拦截，防止追踪像素。"))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .task { notificationPermission = await MailNotifier.authorizationStatus() }
    }

    private var accountsForm: some View {
        Group {
            Section(L10n.t("账号")) {
                if store.accounts.isEmpty {
                    Text(L10n.t("还没有账号"))
                        .foregroundStyle(.secondary)
                }
                ForEach(store.accounts) { account in
                    Button {
                        #if os(macOS)
                        editing = account
                        #else
                        store.showSettings = false
                        store.editingAccount = account
                        #endif
                    } label: {
                        HStack(spacing: 12) {
                            Circle()
                                .fill(ChckColor.hex(account.color).opacity(0.18))
                                .frame(width: 28, height: 28)
                                .overlay {
                                    Text(account.initial)
                                        .font(.system(size: 12, weight: .semibold))
                                        .foregroundStyle(ChckColor.hex(account.color))
                                }
                            VStack(alignment: .leading, spacing: 2) {
                                Text(account.title)
                                if account.title != account.email {
                                    Text(account.email)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            Spacer()
                            if account.isAuthRequired {
                                Text(L10n.t("需重新登录"))
                                    .font(.caption)
                                    .foregroundStyle(ChckColor.danger)
                            }
                            Image(systemName: "chevron.right")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .buttonStyle(.plain)
                }
                Button(L10n.t("添加账号…")) {
                    store.showSettings = false
                    store.showAddAccount = true
                }
            }
            if !store.accounts.isEmpty {
                Section {
                    Button(L10n.t("清除全部账号"), role: .destructive) {
                        confirmClear = true
                    }
                }
            }
        }
        .confirmationDialog(L10n.t("清除全部账号？"), isPresented: $confirmClear, titleVisibility: .visible) {
            Button(L10n.t("清除全部"), role: .destructive) {
                Task { await store.removeAllAccounts() }
            }
        } message: {
            Text(L10n.t("账号和本地邮件缓存都会删除，不可恢复。"))
        }
    }

    private var aboutForm: some View {
        Section(L10n.t("关于")) {
            BrandMark(size: 72)
                .frame(maxWidth: .infinity)
                .listRowBackground(Color.clear)
            LabeledContent(L10n.t("产品"), value: ChckBrand.product)
            LabeledContent(L10n.t("品牌"), value: ChckBrand.product)
            LabeledContent("Bundle ID", value: ChckBrand.bundleID)
            Link(L10n.t("隐私政策"), destination: ChckBrand.privacy)
            Link(L10n.t("官网"), destination: ChckBrand.site)
        }
    }

    private func editSheet(_ account: MailAccount) -> some View {
        EditAccountSheet(store: store, account: account) {
            editing = nil
        }
    }
}

private enum SettingsPane: String, CaseIterable, Identifiable, Hashable {
    case accounts
    case appearance
    case mail
    case about

    var id: String { rawValue }

    var title: String {
        switch self {
        case .accounts: L10n.t("账号")
        case .appearance: L10n.t("外观")
        case .mail: L10n.t("邮件")
        case .about: L10n.t("关于")
        }
    }

    var systemImage: String {
        switch self {
        case .accounts: "person.crop.circle"
        case .appearance: "paintbrush"
        case .mail: "envelope"
        case .about: "info.circle"
        }
    }
}
