import ChckDesign
import ChckAppCore
import XCTest

@MainActor
final class MailSafetyTests: XCTestCase {
    func testMailLinksCannotLaunchFilesOrApplications() {
        for raw in ["file:///etc/passwd", "javascript:alert(1)", "data:text/html,test", "imyemailcloud:compose", "x-apple.systempreferences:com.apple.preference.security"] {
            XCTAssertFalse(MailContentPolicy.allowsExternalURL(URL(string: raw)!))
        }
        for raw in ["https://imy.email", "HTTP://example.com", "mailto:alice@example.com"] {
            XCTAssertTrue(MailContentPolicy.allowsExternalURL(URL(string: raw)!))
        }
    }

    func testArchiveMovesAndRetainsBody() async throws {
        let engine = PreviewEngine()
        let store = MailboxStore(engine: engine)
        await store.bootstrap()
        let row = try XCTUnwrap(store.messages.first)
        store.selectedMessage = row
        store.archiveSelected()
        for _ in 0..<40 where store.messages.contains(where: { $0.id == row.id }) {
            try await Task.sleep(for: .milliseconds(10))
        }
        let inbox = try await engine.messages(folderId: row.folderId)
        let archive = try await engine.messages(folderId: "preview:Archive")
        let body = try await engine.body(messageId: row.id)
        XCTAssertFalse(inbox.contains { $0.id == row.id })
        XCTAssertTrue(archive.contains { $0.id == row.id })
        XCTAssertFalse(body.text.isEmpty)
    }

    func testMissingArchiveNeverDeletes() async throws {
        let engine = PreviewEngine()
        let store = MailboxStore(engine: engine)
        store.accounts = try await engine.accounts()
        store.messages = try await engine.messages(folderId: "preview:INBOX")
        store.folders = try await engine.folders(accountId: "preview").filter { $0.role != "archive" }
        let row = try XCTUnwrap(store.messages.first)
        store.selectedMessage = row
        store.archiveSelected()
        for _ in 0..<40 where store.status != L10n.t("此账号没有归档文件夹，邮件已保留") {
            try await Task.sleep(for: .milliseconds(10))
        }
        XCTAssertTrue(store.status == L10n.t("此账号没有归档文件夹，邮件已保留"))
        let rows = try await engine.messages(folderId: row.folderId)
        XCTAssertTrue(rows.contains { $0.id == row.id })
    }

    func testLateBodyCannotReplaceNewSelection() async throws {
        let engine = DelayedBodyEngine()
        let store = MailboxStore(engine: engine)
        await store.bootstrap()
        let rows = store.messages
        let first = Task { await store.open(rows[0]) }
        await engine.waitForFirstRequest()
        XCTAssertNil(store.body)
        await store.open(rows[1])
        await engine.releaseFirstRequest()
        await first.value
        XCTAssertEqual(store.selectedMessage?.id, rows[1].id)
        XCTAssertEqual(store.body?.id, rows[1].id)
        XCTAssertFalse(store.isLoadingBody)
    }
}

private actor DelayedBodyEngine: MailEngineClient {
    private let preview = PreviewEngine()
    private var continuation: CheckedContinuation<Void, Never>?
    func waitForFirstRequest() async {
        while continuation == nil { await Task.yield() }
    }
    func releaseFirstRequest() { continuation?.resume(); continuation = nil }
    func body(messageId: String) async throws -> MailBody {
        if messageId == "preview:INBOX:1" {
            await withCheckedContinuation { continuation = $0 }
        }
        return try await preview.body(messageId: messageId)
    }
    func accounts() async throws -> [MailAccount] { try await preview.accounts() }
    func providers() async throws -> [MailProvider] { try await preview.providers() }
    func folders(accountId: String) async throws -> [MailFolder] { try await preview.folders(accountId: accountId) }
    func sync(folderId: String) async throws {}
    func messages(folderId: String) async throws -> [MailRow] { try await preview.messages(folderId: folderId) }
    func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount { throw CliError.failed("not used") }
    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String { throw CliError.failed("not used") }
}
