import ChckAppCore
import XCTest

final class NewMailTrackerTests: XCTestCase {
    func testFirstMailAfterEmptyInboxAndNoRepeat() {
        var tracker = NewMailTracker()
        XCTAssertTrue(tracker.observe(accountID: "a", rows: []).isEmpty)
        let mail = MailRow(id: "a:INBOX:1", subject: "hello", from: "sender", snippet: "", unread: true, accountId: "a")
        XCTAssertEqual(tracker.observe(accountID: "a", rows: [mail]).count, 1)
        XCTAssertTrue(tracker.observe(accountID: "a", rows: [mail]).isEmpty)
        XCTAssertTrue(tracker.observe(accountID: "a", rows: []).isEmpty)
        XCTAssertTrue(tracker.observe(accountID: "a", rows: [mail]).isEmpty)
    }
    func testInitialHistoryDoesNotNotifyAndAccountsAreIndependent() {
        var tracker = NewMailTracker()
        let mail = MailRow(id: "b:INBOX:1", subject: "old", from: "sender", snippet: "", unread: true, accountId: "b")
        XCTAssertTrue(tracker.observe(accountID: "a", rows: [mail]).isEmpty)
        XCTAssertTrue(tracker.observe(accountID: "b", rows: [mail]).isEmpty)
        let fresh = MailRow(id: "a:INBOX:1", subject: "new", from: "sender", snippet: "", unread: true, accountId: "a")
        XCTAssertEqual(tracker.observe(accountID: "a", rows: [mail, fresh]).map(\.id), [fresh.id])
    }
}
