import ChckAppCore
import ChckDesign
import SwiftUI

struct EditAccountSheet: View {
    @Bindable var store: MailboxStore
    let account: MailAccount
    var onClose: () -> Void = {}
    @Environment(\.dismiss) private var dismiss
    @State private var settings = AccountSettings()
    @State private var password = ""
    @State private var busy = false
    @State private var loaded = false
    @State private var error = ""
    @State private var probe: ConnectProbe?
    @State private var confirmDelete = false

    var body: some View {
        NavigationStack {
            Form {
                Section(L10n.t("账号")) {
                    LabeledContent(L10n.t("邮箱"), value: account.email)
                    TextField(L10n.t("显示名"), text: $settings.displayName)
                    SecureField(L10n.t("新密码 / 授权码（不改请留空）"), text: $password)
                }
                AccountServerFields(settings: $settings)
                Section(L10n.t("连接测试")) {
                    Button(L10n.t("测试 IMAP 和 SMTP")) { Task { await test() } }
                        .disabled(!loaded || busy)
                    if busy { ProgressView(L10n.t("正在处理…")) }
                    if let probe { AccountProbeResults(probe: probe) }
                }
                if !error.isEmpty {
                    Section { Text(error).foregroundStyle(ChckColor.danger).textSelection(.enabled) }
                }
                Section {
                    Text(L10n.t("保存后，可点击主窗口底部的“发件队列”重试失败邮件。"))
                        .font(.caption).foregroundStyle(.secondary)
                    Button(L10n.t("删除账号"), role: .destructive) { confirmDelete = true }
                }
            }
            .formStyle(.grouped)
            .disabled(busy)
            .navigationTitle(L10n.t("编辑账号"))
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.t("取消")) { close() }.disabled(busy)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(L10n.t("保存")) { Task { await save() } }.disabled(!loaded || busy)
                }
            }
            .confirmationDialog(L10n.t("删除这个账号？"), isPresented: $confirmDelete, titleVisibility: .visible) {
                Button(L10n.t("删除账号"), role: .destructive) {
                    Task { await store.removeAccount(account); close() }
                }
            } message: { Text(L10n.t("将删除 \(account.email) 及其本地邮件缓存。")) }
        }
        #if os(macOS)
        .frame(minWidth: 620, idealWidth: 680, minHeight: 660, idealHeight: 740)
        #endif
        .task {
            busy = true
            do { settings = try await store.engine.accountSettings(id: account.id); loaded = true }
            catch { self.error = MailboxStore.displayError(error) }
            busy = false
        }
        .onChange(of: settings) { _, _ in probe = nil }
        .onChange(of: password) { _, _ in probe = nil }
    }

    private func close() {
        store.editingAccount = nil
        onClose()
        dismiss()
    }

    private func test() async {
        error = ""
        if let message = settings.validationError { error = message; return }
        busy = true
        defer { busy = false }
        do { probe = try await store.engine.testAccountSettings(settings, id: account.id, password: password) }
        catch { self.error = MailboxStore.displayError(error) }
    }

    private func save() async {
        error = ""
        if let message = settings.validationError { error = message; return }
        busy = true
        let saved = await store.saveAccountSettings(settings, id: account.id, password: password)
        busy = false
        if saved { close() } else { error = store.status }
    }
}
