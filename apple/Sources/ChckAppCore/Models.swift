import ChckDesign
import SwiftUI
import Foundation

public struct MailAccount: Identifiable, Hashable, Sendable, Codable {
    public let id: String
    public var email: String
    public var displayName: String
    public var providerId: String
    public var mode: String
    public var color: String
    public var state: String

    public init(
        id: String,
        email: String,
        displayName: String,
        providerId: String,
        mode: String = "standard",
        color: String = "#3D6BFE",
        state: String = "online"
    ) {
        self.id = id
        self.email = email
        self.displayName = displayName
        self.providerId = providerId
        self.mode = mode
        self.color = color
        self.state = state
    }

    public var isAuthRequired: Bool { state == "authRequired" }
    public var isEnhanced: Bool { mode == "enhanced" }
    public var title: String {
        let name = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        let local = email.split(separator: "@").first.map(String.init) ?? ""
        if !name.isEmpty, name != email, name != local {
            return name
        }
        return email.isEmpty ? name : email
    }

    public var initial: String {
        String(title.trimmingCharacters(in: .whitespacesAndNewlines).prefix(1)).uppercased()
    }
}

public struct MailFolder: Identifiable, Hashable, Sendable, Codable {
    public let id: String
    public var accountId: String
    public var path: String
    public var role: String
    public var unread: UInt32
    public var total: UInt32

    public init(id: String, path: String, role: String, unread: UInt32, accountId: String = "", total: UInt32 = 0) {
        self.id = id
        self.accountId = accountId
        self.path = path
        self.role = role
        self.unread = unread
        self.total = total
    }

    public var title: String {
        switch role {
        case "inbox": L10n.t("收件箱")
        case "sent": L10n.t("已发送")
        case "drafts": L10n.t("草稿")
        case "trash": L10n.t("废纸篓")
        case "junk": L10n.t("垃圾邮件")
        case "archive": L10n.t("归档")
        case "flagged": L10n.t("星标")
        case "all": L10n.t("全部邮件")
        default: path
        }
    }

    public var systemImage: String {
        switch role {
        case "inbox": "tray"
        case "sent": "paperplane"
        case "drafts": "doc"
        case "trash": "trash"
        case "junk": "xmark.bin"
        case "archive": "archivebox"
        case "flagged": "star"
        default: "folder"
        }
    }
}

public struct MailFlags: Hashable, Sendable, Codable {
    public var seen: Bool
    public var flagged: Bool
    public var answered: Bool
    public var draft: Bool
    public var deleted: Bool

    public init(seen: Bool = false, flagged: Bool = false, answered: Bool = false, draft: Bool = false, deleted: Bool = false) {
        self.seen = seen
        self.flagged = flagged
        self.answered = answered
        self.draft = draft
        self.deleted = deleted
    }
}

public struct MailRow: Identifiable, Hashable, Sendable {
    public let id: String
    public var accountId: String
    public var folderId: String
    public var threadId: String?
    public var subject: String
    public var from: String
    public var fromName: String?
    public var dateUnix: Int64
    public var snippet: String
    public var unread: Bool
    public var flagged: Bool
    public var answered: Bool
    public var hasAttachments: Bool
    public var labels: [String]

    public init(
        id: String,
        subject: String,
        from: String,
        snippet: String,
        unread: Bool,
        accountId: String = "",
        folderId: String = "",
        threadId: String? = nil,
        fromName: String? = nil,
        dateUnix: Int64 = 0,
        flagged: Bool = false,
        answered: Bool = false,
        hasAttachments: Bool = false,
        labels: [String] = []
    ) {
        self.id = id
        self.accountId = accountId
        self.folderId = folderId
        self.threadId = threadId
        self.subject = subject
        self.from = from
        self.fromName = fromName
        self.dateUnix = dateUnix
        self.snippet = snippet
        self.unread = unread
        self.flagged = flagged
        self.answered = answered
        self.hasAttachments = hasAttachments
        self.labels = labels
    }

    public var flags: MailFlags {
        MailFlags(seen: !unread, flagged: flagged, answered: answered)
    }

    public var senderLabel: String {
        if let fromName, !fromName.isEmpty { return fromName }
        return from
    }

    public var replyAddress: String {
        MailAddress.canonical(from)
    }

    public var dateLabel: String {
        guard dateUnix > 0 else { return "" }
        let date = Date(timeIntervalSince1970: TimeInterval(dateUnix))
        let cal = Calendar.current
        if cal.isDateInToday(date) {
            return date.formatted(date: .omitted, time: .shortened)
        }
        return date.formatted(date: .abbreviated, time: .omitted)
    }
}

public struct MailBody: Sendable {
    public var id: String
    public var text: String
    public var html: String
    public var remoteBlocked: UInt32

    public init(text: String, html: String, id: String = "", remoteBlocked: UInt32 = 0) {
        self.id = id
        self.text = text
        self.html = html
        self.remoteBlocked = remoteBlocked
    }
}

public struct MailProvider: Identifiable, Hashable, Sendable {
    public let id: String
    public var displayName: String
    public var imapHost: String
    public var imapPort: UInt16
    public var smtpHost: String
    public var smtpPort: UInt16
    public var smtpStarttls: Bool
    public var authKind: String
    public var helpURL: String?
    public var domains: [String]
    public var quirks: [String]

    public init(
        id: String,
        displayName: String,
        imapHost: String,
        authKind: String,
        helpURL: String? = nil,
        imapPort: UInt16 = 993,
        smtpHost: String = "",
        smtpPort: UInt16 = 465,
        smtpStarttls: Bool = false,
        domains: [String] = [],
        quirks: [String] = []
    ) {
        self.id = id
        self.displayName = displayName
        self.imapHost = imapHost
        self.imapPort = imapPort
        self.smtpHost = smtpHost.isEmpty ? imapHost.replacingOccurrences(of: "imap.", with: "smtp.") : smtpHost
        self.smtpPort = smtpPort
        self.smtpStarttls = smtpStarttls
        self.authKind = authKind
        self.helpURL = helpURL
        self.domains = domains
        self.quirks = quirks
    }

    public var isInternational: Bool {
        ["gmail", "outlook", "yahoo", "icloud", "aol", "yandex", "mailru"].contains(id)
    }

    /// Enterprise mail uses the customer's own domain, not the provider's server domain.
    public var emailDomains: [String] {
        ["exmail", "aliyun-qiye"].contains(id) ? [] : domains
    }

    public var credentialTitle: String {
        switch authKind {
        case "oauth2": L10n.t("OAuth2 / 应用密码")
        case "authcode": L10n.t("授权码")
        case "app-password": L10n.t("应用专用密码")
        default: L10n.t("密码")
        }
    }

    public var credentialHint: String {
        switch authKind {
        case "oauth2": L10n.t("系统浏览器完成 PKCE；也可填应用专用密码")
        case "authcode": L10n.t("请使用客户端授权码，不要使用登录密码")
        case "app-password": L10n.t("请在服务商后台生成应用专用密码")
        default: L10n.t("账号密码经 TLS 提交，不会写入邮件库")
        }
    }
}

public struct ProbeStep: Identifiable, Hashable, Sendable {
    public var kind: String
    public var status: String
    public var detail: String
    public var id: String { kind }

    public init(kind: String, status: String, detail: String) {
        self.kind = kind
        self.status = status
        self.detail = detail
    }

    public var title: String {
        switch kind {
        case "dns": "DNS"
        case "tcp": "TCP"
        case "tls": "TLS"
        case "certificate": L10n.t("证书")
        case "auth": L10n.t("鉴权")
        case "folders": L10n.t("IMAP 文件夹发现")
        case "smtp": L10n.t("SMTP 发信认证")
        default: kind
        }
    }
}

public struct ConnectProbe: Sendable {
    public var steps: [ProbeStep]
    public init(steps: [ProbeStep] = []) { self.steps = steps }
    public var success: Bool { !steps.isEmpty && steps.allSatisfy { $0.status == "ok" } }
}

public struct OutboxItem: Identifiable, Hashable, Sendable {
    public var draftId: String
    public var accountId: String
    public var state: String
    public var retryCount: UInt32
    public var error: String?
    public var id: String { draftId }

    public init(draftId: String, accountId: String, state: String, retryCount: UInt32 = 0, error: String? = nil) {
        self.draftId = draftId
        self.accountId = accountId
        self.state = state
        self.retryCount = retryCount
        self.error = error
    }
}

public enum SidebarItem: Hashable, Sendable {
    case unified
    case starred
    case snooze
    case folder(String)

    public var id: String {
        switch self {
        case .unified: "unified"
        case .starred: "starred"
        case .snooze: "snooze"
        case .folder(let id): "folder:\(id)"
        }
    }
}

public enum MailRoute: Hashable, Sendable {
    case mailbox(SidebarItem)
    case message(String)
}

public enum MailAppearance: String, CaseIterable, Identifiable, Sendable {
    public var colorScheme: ColorScheme? {
        switch self {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
    case system
    case light
    case dark
    public var id: String { rawValue }
    public var title: String {
        switch self {
        case .system: L10n.t("跟随系统")
        case .light: L10n.t("浅色")
        case .dark: L10n.t("深色")
        }
    }
}

public enum MessageFilter: String, CaseIterable, Identifiable, Sendable {
    case all
    case unread
    case starred
    case attachments

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .all: L10n.t("全部")
        case .unread: L10n.t("未读")
        case .starred: L10n.t("星标")
        case .attachments: L10n.t("附件")
        }
    }
}

public enum MailAddress {
    public static func split(_ raw: String) -> [String] {
        raw.split { $0 == "," || $0 == ";" }
            .map { canonical(String($0)) }
            .filter { !$0.isEmpty }
    }

    public static func canonical(_ raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if let start = trimmed.lastIndex(of: "<"), let end = trimmed.lastIndex(of: ">"), start < end {
            return String(trimmed[trimmed.index(after: start)..<end]).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return trimmed
    }
}
