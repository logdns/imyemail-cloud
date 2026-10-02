import ChckDesign
import Foundation

/// Editable server configuration. Reading settings never returns stored passwords.
public struct AccountSettings: Codable, Sendable, Equatable {
    public var email = ""
    public var displayName = ""
    public var username = ""
    public var imapHost = ""
    public var imapPort: UInt16 = 993
    public var imapStarttls = false
    public var smtpHost = ""
    public var smtpPort: UInt16 = 465
    public var smtpStarttls = false
    public var acceptInvalidCerts = false
    public init() {}

    public mutating func apply(_ provider: MailProvider) {
        imapHost = provider.imapHost
        imapPort = provider.imapPort
        smtpHost = provider.smtpHost
        smtpPort = provider.smtpPort
        smtpStarttls = provider.smtpStarttls
    }

    public var validationError: String? {
        if !email.contains("@") || email.hasPrefix("@") { return L10n.t("请输入完整邮箱地址") }
        for host in [imapHost, smtpHost] {
            if host.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return L10n.t("请填写 IMAP 和 SMTP 主机") }
            if host.contains("://") || host.contains(where: { $0.isWhitespace }) || host.contains("/") { return L10n.t("服务器请填写主机名，不要填写网址或端口") }
        }
        if imapPort == 0 || smtpPort == 0 { return L10n.t("端口必须在 1–65535 之间") }
        return nil
    }

    func requestJSON(id: String? = nil, password: String = "") throws -> String {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        var object = try JSONSerialization.jsonObject(with: encoder.encode(self)) as! [String: Any]
        if username.isEmpty { object["username"] = email }
        if let id { object["id"] = id }
        if !password.isEmpty { object["password"] = password }
        return String(decoding: try JSONSerialization.data(withJSONObject: object), as: UTF8.self)
    }

    static func fromJSON(_ json: String) throws -> AccountSettings {
        guard let object = JSONCoding.object(json) as? [String: Any] else { throw CliError.failed("invalid account settings") }
        var value = AccountSettings()
        value.email = object["email"] as? String ?? ""
        value.displayName = object["display_name"] as? String ?? ""
        value.username = object["username"] as? String ?? ""
        value.imapHost = object["imap_host"] as? String ?? ""
        value.imapPort = (object["imap_port"] as? NSNumber)?.uint16Value ?? 993
        value.imapStarttls = object["imap_starttls"] as? Bool ?? false
        value.smtpHost = object["smtp_host"] as? String ?? ""
        value.smtpPort = (object["smtp_port"] as? NSNumber)?.uint16Value ?? 465
        value.smtpStarttls = object["smtp_starttls"] as? Bool ?? false
        value.acceptInvalidCerts = object["accept_invalid_certs"] as? Bool ?? false
        return value
    }
}
