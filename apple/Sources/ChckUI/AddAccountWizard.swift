import ChckAppCore
import ChckDesign
import SwiftUI

public struct AddAccountWizard: View {
    @Bindable var store: MailboxStore

    public init(store: MailboxStore) {
        self.store = store
    }

    public var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 16) {
                stepHeader
                Group {
                    switch store.wizard.step {
                    case 1: emailStep
                    case 2: credentialStep
                    case 3: probeStep
                    default: personalizeStep
                    }
                }
                Spacer()
                if !store.wizard.error.isEmpty {
                    Text(store.wizard.error)
                        .foregroundStyle(ChckColor.danger)
                        .font(.caption)
                }
                HStack {
                    if store.wizard.step > 1 {
                        Button(L10n.t("上一步")) { store.wizard.step -= 1 }
                            .disabled(store.wizard.probing || store.wizard.saving)
                    }
                    Spacer()
                    Button(store.wizard.step == 4 ? L10n.t("完成") : L10n.t("继续")) {
                        Task { await advance() }
                    }
                    .keyboardShortcut(.defaultAction)
                    .disabled(store.wizard.probing || store.wizard.saving ||
                        (store.wizard.step == 1 && (!store.wizard.email.contains("@") || store.wizard.email.hasPrefix("@") || store.wizard.email.hasSuffix("@"))) ||
                        (store.wizard.step == 3 && store.wizard.probe?.success != true))
                }
            }
            .padding(24)
            .frame(maxWidth: 880, maxHeight: 640)
            #if os(macOS)
            .frame(minWidth: 720, minHeight: 480)
            #endif
            .navigationTitle(L10n.t("添加账号"))
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.t("取消")) { store.showAddAccount = false }
                        .disabled(store.wizard.probing || store.wizard.saving)
                }
            }
        }
        .task {
            if store.wizard.email.isEmpty { store.wizard.email = store.draftEmail }
            if store.providers.isEmpty {
                store.providers = (try? await store.engine.providers()) ?? []
            }
        }
    }

    private var stepHeader: some View {
        HStack(spacing: 8) {
            ForEach(1...4, id: \.self) { n in
                Circle()
                    .fill(n <= store.wizard.step ? ChckColor.brand : Color.secondary.opacity(0.25))
                    .frame(width: 8, height: 8)
            }
            Text([L10n.t("输入邮箱"), L10n.t("凭据"), L10n.t("连接测试"), L10n.t("个性化")][store.wizard.step - 1])
                .foregroundStyle(.secondary)
        }
    }

    private var emailStep: some View {
        VStack(alignment: .leading, spacing: 16) {
            TextField(L10n.t("邮箱地址"), text: Bindable(store.wizard).email)
                .textFieldStyle(.roundedBorder)
                .textContentType(.username)
                #if os(iOS)
                .keyboardType(.emailAddress)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif
            if let provider = store.wizard.selectedProvider {
                if provider.emailDomains.isEmpty {
                    Text(L10n.t("请输入完整企业邮箱地址，例如 name@company.com"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    HStack {
                        Text(L10n.t("邮箱后缀"))
                            .foregroundStyle(.secondary)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack {
                                ForEach(provider.emailDomains, id: \.self) { domain in
                                    Button("@\(domain)") { store.wizard.selectEmailDomain(domain) }
                                        .buttonStyle(.bordered)
                                }
                            }
                        }
                    }
                    .font(.caption)
                }
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    Text(L10n.t("国际"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    providerGrid(store.providers.filter(\.isInternational))
                    Text(L10n.t("国内"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    providerGrid(store.providers.filter { !$0.isInternational })
                }
            }
            Button(L10n.t("自定义 IMAP/SMTP")) {
                store.wizard.selectCustomServer()
            }
        }
    }

    private func providerGrid(_ items: [MailProvider]) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 140), spacing: 8)], spacing: 8) {
            ForEach(items) { provider in
                Button {
                    store.wizard.selectProvider(provider)
                } label: {
                    VStack(spacing: 4) {
                        Text(provider.displayName)
                        Text(provider.emailDomains.first.map { "@\($0)" } ?? L10n.t("企业自有域名"))
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                        .font(.caption)
                        .frame(maxWidth: .infinity)
                        .padding(8)
                        .background(store.wizard.selectedProvider?.id == provider.id ? ChckColor.brand.opacity(0.15) : Color.secondary.opacity(0.08))
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .buttonStyle(.plain)
            }
        }
    }

    private var credentialStep: some View {
        Form {
            LabeledContent(L10n.t("邮箱"), value: store.wizard.email)
            if let provider = matchedProvider {
                LabeledContent(L10n.t("服务商"), value: provider.displayName)
                Text(provider.credentialHint)
                    .foregroundStyle(.secondary)
                    .font(.caption)
                if let help = provider.helpURL, let url = URL(string: help) {
                    Link(L10n.t("获取\(provider.credentialTitle)"), destination: url)
                }
            }
            SecureField(matchedProvider?.credentialTitle ?? L10n.t("密码 / 授权码"), text: Bindable(store.wizard).password)
            AccountServerFields(settings: Bindable(store.wizard).settings)
        }
        .formStyle(.grouped)
    }

    private var probeStep: some View {
        VStack(alignment: .leading, spacing: 12) {
            if store.wizard.probing {
                ProgressView(L10n.t("正在测试连接…"))
            }
            ForEach(store.wizard.probe?.steps ?? []) { step in
                HStack {
                    Image(systemName: icon(for: step.status))
                        .foregroundStyle(color(for: step.status))
                    Text(step.title)
                    Spacer()
                    Text(step.detail)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            if let probe = store.wizard.probe, !probe.success {
                Text(hintForFailure)
                    .foregroundStyle(ChckColor.danger)
            }
        }
    }

    private var personalizeStep: some View {
        Form {
            TextField(L10n.t("显示名"), text: Bindable(store.wizard).displayName)
            LabeledContent(L10n.t("同步范围"), value: L10n.t("近 30 天"))
            Text(L10n.t("签名可在设置中稍后添加。通知权限将在首次收到邮件时请求。"))
                .foregroundStyle(.secondary)
                .font(.caption)
        }
        .formStyle(.grouped)
    }

    private var matchedProvider: MailProvider? {
        if store.wizard.usesCustomServer { return nil }
        if let selected = store.wizard.selectedProvider { return selected }
        let domain = store.wizard.email.split(separator: "@").last.map(String.init)?.lowercased() ?? ""
        return store.providers.first { $0.domains.contains(domain) }
    }

    private var hintForFailure: String {
        let domain = store.wizard.email.split(separator: "@").last.map(String.init)?.lowercased() ?? ""
        if domain.hasSuffix("qq.com") { return L10n.t("QQ 邮箱请使用授权码而非 QQ 密码") }
        if domain.hasSuffix("163.com") || domain.hasSuffix("126.com") { return L10n.t("网易邮箱请使用客户端授权码，并向导会自动发送 IMAP ID") }
        return L10n.t("请检查主机、端口与凭据后重试")
    }

    private func icon(for status: String) -> String {
        switch status {
        case "ok": "checkmark.circle.fill"
        case "failed": "xmark.circle.fill"
        default: "minus.circle"
        }
    }

    private func color(for status: String) -> Color {
        switch status {
        case "ok": ChckColor.success
        case "failed": ChckColor.danger
        default: .secondary
        }
    }

    private func advance() async {
        let wizard = store.wizard
        wizard.error = ""
        if wizard.step == 1 {
            wizard.email = wizard.email.trimmingCharacters(in: .whitespacesAndNewlines)
            if wizard.settings.imapHost.isEmpty, let provider = matchedProvider {
                wizard.settings.apply(provider)
            }
            wizard.settings.email = wizard.email
            wizard.step = 2
        } else if wizard.step == 2 {
            if let error = wizard.settings.validationError { wizard.error = error; return }
            if wizard.password.isEmpty { wizard.error = L10n.t("请输入密码或客户端授权码"); return }
            wizard.step = 3
            wizard.probing = true
            wizard.probe = nil
            do {
                wizard.probe = try await store.engine.testAccountSettings(wizard.settings, id: nil, password: wizard.password)
            } catch { wizard.error = MailboxStore.displayError(error) }
            wizard.probing = false
        } else if wizard.step == 3 {
            guard wizard.probe?.success == true else { return }
            wizard.step = 4
            if wizard.displayName.isEmpty { wizard.displayName = wizard.email }
        } else {
            wizard.settings.displayName = wizard.displayName
            wizard.saving = true
            let saved = await store.saveAccountSettings(wizard.settings, id: nil, password: wizard.password)
            wizard.saving = false
            if !saved { wizard.error = store.status }
        }
    }
}

public struct AddAccountSheet: View {
    @Bindable var store: MailboxStore
    public init(store: MailboxStore) { self.store = store }
    public var body: some View { AddAccountWizard(store: store) }
}
