import ChckDesign
import Foundation

/// A broken installation must never present sample mail as a working account.
struct UnavailableEngine: MailEngineClient {
    private var error: CliError { .failed(L10n.t("邮件服务未能启动，请重新打开或重新安装应用。")) }
    func accounts() async throws -> [MailAccount] { throw error }
    func providers() async throws -> [MailProvider] { throw error }
    func folders(accountId: String) async throws -> [MailFolder] { throw error }
    func sync(folderId: String) async throws { throw error }
    func messages(folderId: String) async throws -> [MailRow] { throw error }
    func body(messageId: String) async throws -> MailBody { throw error }
    func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount { throw error }
    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String { throw error }
    func saveDraft(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String { throw error }
    func saveAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> MailAccount { throw error }
    func testAccountSettings(_ settings: AccountSettings, id: String?, password: String) async throws -> ConnectProbe { throw error }
    func testAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool) async throws -> ConnectProbe { throw error }
    func setFlags(messageId: String, flags: MailFlags) async throws { throw error }
    func deleteMessage(messageId: String) async throws { throw error }
    func undoSend(draftId: String) async throws { throw error }
}
