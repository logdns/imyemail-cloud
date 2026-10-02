import ChckDesign
import Foundation

public protocol MailEngineClient: Sendable {
    func accountSettings(id: String) async throws -> AccountSettings
    func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount
    func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe
    func clearFailedQueue() async throws -> Int
    func retryFailedSends() async throws

    func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount
    func updateAccount(id: String, email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount
    func removeAccount(id: String) async throws
    func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe
    func accounts() async throws -> [MailAccount]
    func providers() async throws -> [MailProvider]
    func folders(accountId: String) async throws -> [MailFolder]
    func sync(folderId: String) async throws
    func messages(folderId: String) async throws -> [MailRow]
    func unifiedInbox() async throws -> [MailRow]
    func cachedBody(messageId: String) async throws -> MailBody?
    func body(messageId: String) async throws -> MailBody
    func search(query: String) async throws -> [String]
    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String?) async throws -> String
    func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String?) async throws -> String
    func setFlags(messageId: String, flags: MailFlags) async throws
    func deleteMessage(messageId: String) async throws
    func moveMessage(messageId: String, destinationFolderId: String) async throws
    func undoSend(draftId: String) async throws
    func outbox() async throws -> [OutboxItem]
    func flushDueSends() async throws -> UInt32
    func tick(accountId: String?, idle: Bool) async throws
}

public extension MailEngineClient {
    func clearFailedQueue() async throws -> Int { 0 }
    func cachedBody(messageId: String) async throws -> MailBody? { nil }
    func accountSettings(id: String) async throws -> AccountSettings { throw CliError.failed("account settings unsupported") }
    func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount { throw CliError.failed("account settings unsupported") }
    func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe { throw CliError.failed("account settings unsupported") }
    func retryFailedSends() async throws {}

    func updateAccount(id: String, email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount {
        _ = (id, email, host, port, password, insecure, displayName)
        throw CliError.failed("update account unsupported")
    }

    func removeAccount(id: String) async throws {
        _ = id
        throw CliError.failed("remove account unsupported")
    }

    func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe {
        _ = (email, host, port, password, insecure)
        return ConnectProbe(steps: [
            ProbeStep(kind: "dns", status: "ok", detail: "skipped"),
            ProbeStep(kind: "tls", status: "ok", detail: "skipped"),
            ProbeStep(kind: "auth", status: "ok", detail: "skipped"),
        ])
    }

    func unifiedInbox() async throws -> [MailRow] {
        var rows: [MailRow] = []
        for account in try await accounts() {
            for folder in try await folders(accountId: account.id) where folder.role == "inbox" {
                rows.append(contentsOf: try await messages(folderId: folder.id))
            }
        }
        return rows
    }

    func search(query: String) async throws -> [String] {
        let q = query.lowercased()
        return try await unifiedInbox().filter {
            $0.subject.lowercased().contains(q) || $0.from.lowercased().contains(q) || $0.snippet.lowercased().contains(q)
        }.map(\.id)
    }

    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String) async throws -> String {
        try await send(accountId: accountId, to: to, cc: cc, bcc: bcc, subject: subject, body: body, html: nil)
    }

    func send(accountId: String, to: String, subject: String, body: String) async throws -> String {
        try await send(accountId: accountId, to: to, cc: "", bcc: "", subject: subject, body: body)
    }

    func saveDraft(accountId: String, to: String, subject: String, body: String) async throws -> String {
        try await saveDraft(accountId: accountId, to: to, cc: "", bcc: "", subject: subject, body: body)
    }

    func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String {
        _ = (accountId, to, cc, bcc, subject, body)
        throw CliError.failed(L10n.t("当前引擎不支持保存草稿"))
    }

    func setFlags(messageId: String, flags: MailFlags) async throws {
        _ = (messageId, flags)
    }

    func moveMessage(messageId: String, destinationFolderId: String) async throws {
        throw CliError.failed(L10n.t("当前引擎不支持移动邮件"))
    }

    func deleteMessage(messageId: String) async throws {
        _ = messageId
    }

    func undoSend(draftId: String) async throws {
        _ = draftId
    }

    func outbox() async throws -> [OutboxItem] { [] }

    func flushDueSends() async throws -> UInt32 { 0 }

    func tick(accountId: String?, idle: Bool) async throws {
        _ = (accountId, idle)
    }
}
