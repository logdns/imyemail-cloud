import Foundation
import os

public final class PreviewEngine: MailEngineClient, @unchecked Sendable {
    private struct State {
        var settings: [String: AccountSettings] = [:]
        var accounts: [MailAccount]
        var folders: [MailFolder]
        var messages: [MailRow]
        var bodies: [String: MailBody]
        var providers: [MailProvider]
    }

    private let lock: OSAllocatedUnfairLock<State>

    public init() {
        let now = Int64(Date().timeIntervalSince1970)
        lock = OSAllocatedUnfairLock(initialState: State(
            accounts: [
                MailAccount(id: "preview", email: "dev@imyemail.test", displayName: "Dev", providerId: "custom", color: "#3D6BFE"),
            ],
            folders: [
                MailFolder(id: "preview:INBOX", path: "INBOX", role: "inbox", unread: 2, accountId: "preview", total: 3),
                MailFolder(id: "preview:Sent", path: "Sent", role: "sent", unread: 0, accountId: "preview", total: 1),
                MailFolder(id: "preview:Drafts", path: "Drafts", role: "drafts", unread: 0, accountId: "preview", total: 0),
                MailFolder(id: "preview:Archive", path: "Archive", role: "archive", unread: 0, accountId: "preview", total: 0),
                MailFolder(id: "preview:Trash", path: "Trash", role: "trash", unread: 0, accountId: "preview", total: 0),
            ],
            messages: [
                MailRow(id: "preview:INBOX:1", subject: "Hello from testkit", from: "alice@imyemail.test", snippet: "Hello — 测试邮局种子邮件", unread: true, accountId: "preview", folderId: "preview:INBOX", fromName: "Alice", dateUnix: now - 3600),
                MailRow(id: "preview:INBOX:2", subject: "本周摘要", from: "bob@imyemail.test", snippet: "三件事需要你确认", unread: true, accountId: "preview", folderId: "preview:INBOX", fromName: "Bob", dateUnix: now - 7200, flagged: true),
                MailRow(id: "preview:INBOX:3", subject: "发票 INV-1042", from: "billing@imyemail.test", snippet: "附件为 PDF 发票", unread: false, accountId: "preview", folderId: "preview:INBOX", fromName: "Billing", dateUnix: now - 86400, hasAttachments: true),
                MailRow(id: "preview:Sent:1", subject: "Re: Hello from testkit", from: "dev@imyemail.test", snippet: "收到，谢谢", unread: false, accountId: "preview", folderId: "preview:Sent", fromName: "Dev", dateUnix: now - 1800),
            ],
            bodies: [
                "preview:INBOX:1": MailBody(text: "Hello from testkit", html: "<p>Hello from testkit</p>", id: "preview:INBOX:1"),
                "preview:INBOX:2": MailBody(text: "三件事需要你确认。", html: "<p>三件事需要你确认。</p><ol><li>账号同步</li><li>签名</li><li>快捷键</li></ol>", id: "preview:INBOX:2"),
                "preview:INBOX:3": MailBody(text: "附件为 PDF 发票。", html: "<p>附件为 PDF 发票。</p>", id: "preview:INBOX:3", remoteBlocked: 1),
                "preview:Sent:1": MailBody(text: "收到，谢谢", html: "<p>收到，谢谢</p>", id: "preview:Sent:1"),
            ],
            providers: Self.builtinProviders
        ))
    }

    public func accountSettings(id: String) async throws -> AccountSettings {
        lock.withLock { state in
            if let value = state.settings[id] { return value }
            var value = AccountSettings()
            let account = state.accounts.first { $0.id == id }
            value.email = account?.email ?? ""
            value.displayName = account?.displayName ?? ""
            value.imapHost = "imap.imyemail.test"
            value.smtpHost = "smtp.imyemail.test"
            return value
        }
    }

    public func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount {
        if let error = settings.validationError { throw CliError.failed(error) }
        let account: MailAccount
        if let id {
            account = try await updateAccount(id: id, email: settings.email, host: settings.imapHost, port: settings.imapPort, password: password, insecure: settings.acceptInvalidCerts, displayName: settings.displayName)
        } else {
            account = try await addAccount(email: settings.email, host: settings.imapHost, port: settings.imapPort, password: password, insecure: settings.acceptInvalidCerts, displayName: settings.displayName)
        }
        lock.withLock { $0.settings[account.id] = settings }
        return account
    }

    public func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe {
        ConnectProbe(steps: [ProbeStep(kind: "auth", status: "ok", detail: "预览模式：未连接真实服务器"), ProbeStep(kind: "smtp", status: "ok", detail: "预览模式：未连接真实服务器")])
    }

    public func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        _ = (host, port, password, insecure)
        let provider = lock.withLock { state in
            state.providers.first { p in
                p.domains.contains { email.lowercased().hasSuffix($0) }
            }
        }
        let name = displayName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let acc = MailAccount(
            id: email,
            email: email,
            displayName: name,
            providerId: provider?.id ?? "custom",
            color: provider?.id == "qq" ? "#12B7F5" : "#3D6BFE"
        )
        lock.withLock { state in
            state.accounts.append(acc)
            state.folders.append(MailFolder(id: "\(acc.id):INBOX", path: "INBOX", role: "inbox", unread: 0, accountId: acc.id))
        }
        return acc
    }

    public func updateAccount(id: String, email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        _ = (host, port, password, insecure)
        return lock.withLock { state in
            guard let idx = state.accounts.firstIndex(where: { $0.id == id }) else {
                return MailAccount(id: id, email: email, displayName: displayName ?? "", providerId: "custom")
            }
            if !email.isEmpty { state.accounts[idx].email = email }
            if let displayName { state.accounts[idx].displayName = displayName }
            return state.accounts[idx]
        }
    }

    public func removeAccount(id: String) async throws {
        lock.withLock { state in
            state.settings.removeValue(forKey: id)
            state.accounts.removeAll { $0.id == id }
            state.folders.removeAll { $0.accountId == id }
            state.messages.removeAll { $0.accountId == id }
        }
    }

    public func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe {
        _ = (email, host, port, password, insecure)
        return ConnectProbe(steps: [
            ProbeStep(kind: "dns", status: "ok", detail: "preview"),
            ProbeStep(kind: "tcp", status: "ok", detail: "preview"),
            ProbeStep(kind: "tls", status: "ok", detail: "TLS 强制"),
            ProbeStep(kind: "certificate", status: "ok", detail: "preview"),
            ProbeStep(kind: "auth", status: "ok", detail: "preview"),
            ProbeStep(kind: "folders", status: "ok", detail: "INBOX"),
        ])
    }

    public func accounts() async throws -> [MailAccount] {
        lock.withLock { $0.accounts }
    }

    public func providers() async throws -> [MailProvider] {
        lock.withLock { $0.providers }
    }

    public func folders(accountId: String) async throws -> [MailFolder] {
        lock.withLock { $0.folders.filter { $0.accountId == accountId } }
    }

    public func sync(folderId: String) async throws { _ = folderId }

    public func messages(folderId: String) async throws -> [MailRow] {
        lock.withLock { $0.messages.filter { $0.folderId == folderId } }
    }

    public func unifiedInbox() async throws -> [MailRow] {
        lock.withLock { state in
            state.messages.filter { row in
                state.folders.contains { $0.id == row.folderId && $0.role == "inbox" }
            }
        }
    }

    public func body(messageId: String) async throws -> MailBody {
        lock.withLock { $0.bodies[messageId] ?? MailBody(text: "", html: "") }
    }

    public func search(query: String) async throws -> [String] {
        let q = query.lowercased()
        return lock.withLock {
            $0.messages.filter {
                $0.subject.lowercased().contains(q) || $0.from.lowercased().contains(q) || $0.snippet.lowercased().contains(q)
            }.map(\.id)
        }
    }

    public func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        _ = (accountId, to, cc, bcc, subject, body)
        return "preview-draft"
    }

    public func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        _ = (accountId, to, cc, bcc, subject, body)
        return "preview-draft"
    }

    public func setFlags(messageId: String, flags: MailFlags) async throws {
        lock.withLock { state in
            if let idx = state.messages.firstIndex(where: { $0.id == messageId }) {
                state.messages[idx].unread = !flags.seen
                state.messages[idx].flagged = flags.flagged
                state.messages[idx].answered = flags.answered
            }
        }
    }

    public func moveMessage(messageId: String, destinationFolderId: String) async throws {
        try lock.withLock { state in
            guard let index = state.messages.firstIndex(where: { $0.id == messageId }),
                  let folder = state.folders.first(where: { $0.id == destinationFolderId }),
                  folder.accountId == state.messages[index].accountId else {
                throw CliError.failed("目标文件夹不可用")
            }
            state.messages[index].folderId = destinationFolderId
        }
    }

    public func deleteMessage(messageId: String) async throws {
        lock.withLock { $0.messages.removeAll { $0.id == messageId } }
    }

    public func undoSend(draftId: String) async throws { _ = draftId }

    public func outbox() async throws -> [OutboxItem] { [] }

    private static let builtinProviders: [MailProvider] = [
        MailProvider(id: "gmail", displayName: "Gmail", imapHost: "imap.gmail.com", authKind: "oauth2", domains: ["gmail.com", "googlemail.com"]),
        MailProvider(id: "outlook", displayName: "Outlook / Hotmail", imapHost: "outlook.office365.com", authKind: "oauth2", smtpPort: 587, smtpStarttls: true, domains: ["outlook.com", "hotmail.com", "live.com"]),
        MailProvider(id: "yahoo", displayName: "Yahoo", imapHost: "imap.mail.yahoo.com", authKind: "app-password", domains: ["yahoo.com"]),
        MailProvider(id: "icloud", displayName: "iCloud", imapHost: "imap.mail.me.com", authKind: "app-password", smtpPort: 587, smtpStarttls: true, domains: ["icloud.com", "me.com", "mac.com"]),
        MailProvider(id: "aol", displayName: "AOL", imapHost: "imap.aol.com", authKind: "app-password", domains: ["aol.com"]),
        MailProvider(id: "yandex", displayName: "Yandex", imapHost: "imap.yandex.com", authKind: "password", domains: ["yandex.com", "yandex.ru"]),
        MailProvider(id: "mailru", displayName: "Mail.ru", imapHost: "imap.mail.ru", authKind: "password", domains: ["mail.ru"]),
        MailProvider(id: "qq", displayName: "QQ 邮箱 / Foxmail", imapHost: "imap.qq.com", authKind: "authcode", helpURL: "https://imy.email", domains: ["qq.com", "foxmail.com"], quirks: ["imap-id"]),
        MailProvider(id: "exmail", displayName: "腾讯企业邮", imapHost: "imap.exmail.qq.com", authKind: "authcode", domains: ["exmail.qq.com"]),
        MailProvider(id: "netease163", displayName: "网易 163", imapHost: "imap.163.com", authKind: "authcode", domains: ["163.com"], quirks: ["imap-id"]),
        MailProvider(id: "netease126", displayName: "网易 126", imapHost: "imap.126.com", authKind: "authcode", domains: ["126.com"]),
        MailProvider(id: "yeah", displayName: "网易 yeah.net", imapHost: "imap.yeah.net", authKind: "authcode", domains: ["yeah.net"]),
        MailProvider(id: "aliyun", displayName: "阿里邮箱", imapHost: "imap.aliyun.com", authKind: "password", domains: ["aliyun.com"]),
        MailProvider(id: "aliyun-qiye", displayName: "阿里企业邮箱", imapHost: "imap.qiye.aliyun.com", authKind: "password", domains: ["qiye.aliyun.com"]),
        MailProvider(id: "189", displayName: "189 邮箱", imapHost: "imap.189.cn", authKind: "password", domains: ["189.cn"]),
        MailProvider(id: "sohu", displayName: "搜狐邮箱", imapHost: "imap.sohu.com", authKind: "password", domains: ["sohu.com"]),
        MailProvider(id: "sina", displayName: "新浪邮箱", imapHost: "imap.sina.com", authKind: "password", domains: ["sina.com"]),
        MailProvider(id: "139", displayName: "139 邮箱", imapHost: "imap.139.com", authKind: "authcode", domains: ["139.com"]),
        MailProvider(id: "21cn", displayName: "21CN 邮箱", imapHost: "imap.21cn.com", authKind: "authcode", domains: ["21cn.com"]),
        MailProvider(id: "88", displayName: "完美邮箱", imapHost: "imap.88.com", authKind: "password", domains: ["88.com"]),
    ]
}
