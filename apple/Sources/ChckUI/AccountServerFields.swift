import ChckDesign
import ChckAppCore
import SwiftUI

struct AccountServerFields: View {
    @Binding var settings: AccountSettings
    var body: some View {
        Section(L10n.t("收件服务器 · IMAP")) {
            TextField(L10n.t("IMAP 主机"), text: $settings.imapHost)
            ServerPortField(title: L10n.t("IMAP 端口"), port: $settings.imapPort)
            Picker(L10n.t("加密"), selection: $settings.imapStarttls) {
                Text(L10n.t("SSL/TLS（通常 993）")).tag(false)
                Text(L10n.t("STARTTLS（通常 143）")).tag(true)
            }
            .onChange(of: settings.imapStarttls) { _, value in
                if [993, 143].contains(settings.imapPort) { settings.imapPort = value ? 143 : 993 }
            }
        }
        Section(L10n.t("发件服务器 · SMTP")) {
            TextField(L10n.t("SMTP 主机"), text: $settings.smtpHost)
            ServerPortField(title: L10n.t("SMTP 端口"), port: $settings.smtpPort)
            Picker(L10n.t("加密"), selection: $settings.smtpStarttls) {
                Text(L10n.t("SSL/TLS（通常 465）")).tag(false)
                Text(L10n.t("STARTTLS（通常 587）")).tag(true)
            }
            .onChange(of: settings.smtpStarttls) { _, value in
                if [465, 587].contains(settings.smtpPort) { settings.smtpPort = value ? 587 : 465 }
            }
            Text(L10n.t("SMTP 用于新邮件和回复；主机、端口及加密方式请以邮箱服务商提供的信息为准。"))
                .font(.caption).foregroundStyle(.secondary)
        }
        Section(L10n.t("登录与安全")) {
            TextField(L10n.t("登录用户名（默认完整邮箱地址）"), text: $settings.username)
            Text(L10n.t("IMAP 和 SMTP 共用此用户名及密码 / 授权码。当前支持 IMAP 收信，暂不支持 POP3。"))
                .font(.caption).foregroundStyle(.secondary)
            Toggle(L10n.t("接受自签名证书（仅测试邮局）"), isOn: $settings.acceptInvalidCerts)
        }
    }
}

private struct ServerPortField: View {
    let title: String
    @Binding var port: UInt16
    @State private var text = ""
    var body: some View {
        TextField(title, text: $text)
            .onAppear { text = String(port) }
            .onChange(of: text) { _, value in port = UInt16(value) ?? 0 }
            .onChange(of: port) { _, value in
                if value != 0, UInt16(text) != value { text = String(value) }
            }
    }
}

struct AccountProbeResults: View {
    let probe: ConnectProbe
    var body: some View {
        ForEach(probe.steps) { step in
            HStack(alignment: .top) {
                Image(systemName: step.status == "ok" ? "checkmark.circle.fill" : "exclamationmark.circle")
                    .foregroundStyle(step.status == "ok" ? Color.green : Color.red)
                VStack(alignment: .leading) {
                    Text(step.title)
                    Text(step.detail).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }
            }
        }
    }
}
