import ChckAppCore
import XCTest

@MainActor
final class MailExperienceTests: XCTestCase {
    func testInboxArrivalWhileReadingSentAndDisabledNotificationsDoNotReplay() async throws {
        let engine = ExperienceEngine()
        let store = MailboxStore(engine: engine, usingPreview: true)
        store.accounts = try await engine.accounts()
        store.folders = try await engine.folders(accountId: "a")
        store.notificationsEnabled = true
        await store.refresh(userInitiated: false) // Establish empty baseline.
        store.selectedItem = .folder("a:Sent")
        await engine.addMessage("one")
        await store.refresh(userInitiated: false)
        XCTAssertEqual(store.newMailCount, 1)
        XCTAssertEqual(store.unreadCount, 1)
        XCTAssertTrue(store.messages.isEmpty)
        store.dismissNewMail()
        await store.refresh(userInitiated: false)
        XCTAssertEqual(store.newMailCount, 0)
        store.notificationsEnabled = false
        await engine.addMessage("two")
        await store.refresh(userInitiated: false)
        store.notificationsEnabled = true
        await store.refresh(userInitiated: false)
        XCTAssertEqual(store.newMailCount, 0)
    }

    func testWarmReadingDoesNotFetchAgainAndNotificationOpensExactAccount() async throws {
        let engine = ExperienceEngine()
        await engine.addMessage("one")
        let store = MailboxStore(engine: engine, usingPreview: true)
        store.accounts = try await engine.accounts()
        store.folders = try await engine.folders(accountId: "a")
        await store.refresh(userInitiated: false)
        let row = try XCTUnwrap(store.messages.first)
        await store.open(row)
        store.selectedMessage = nil
        store.body = nil
        await store.open(row)
        let count = await engine.bodyFetches
        XCTAssertEqual(count, 1)
        XCTAssertEqual(store.body?.id, row.id)
        XCTAssertFalse(store.isLoadingBody)
        store.selectedMessage = nil
        await store.openNotification(messageID: row.id, accountID: "wrong-account")
        XCTAssertNil(store.selectedMessage)
        await store.openNotification(messageID: row.id, accountID: "a")
        XCTAssertEqual(store.selectedMessage?.id, row.id)
    }
}

private actor ExperienceEngine: MailEngineClient {
    private var rows: [MailRow] = []
    var bodyFetches = 0
    func addMessage(_ id: String) {
        rows.append(MailRow(id: "a:INBOX:\(id)", subject: id, from: "test", snippet: "sample", unread: true, accountId: "a", folderId: "a:INBOX"))
    }
    func accounts() async throws -> [MailAccount] { [MailAccount(id: "a", email: "a@example.test", displayName: "A", providerId: "custom")] }
    func providers() async throws -> [MailProvider] { [] }
    func folders(accountId: String) async throws -> [MailFolder] {
        [MailFolder(id: "a:INBOX", path: "INBOX", role: "inbox", unread: UInt32(rows.count), accountId: "a"),
         MailFolder(id: "a:Sent", path: "Sent", role: "sent", unread: 0, accountId: "a")]
    }
    func sync(folderId: String) async throws {}
    func messages(folderId: String) async throws -> [MailRow] { folderId == "a:INBOX" ? rows : [] }
    func body(messageId: String) async throws -> MailBody {
        bodyFetches += 1
        return MailBody(text: "Sample", html: "<p>Sample</p>", id: messageId)
    }
    func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount { throw CliError.failed("unused") }
    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String { throw CliError.failed("unused") }
}
