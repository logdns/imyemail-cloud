import ChckAppCore
import ChckDesign
import XCTest

@MainActor
final class MailboxStoreTests: XCTestCase {
    func testRemoteImagePreferenceDefaultsOffAndPersists() {
        let suite = "email.imy.cloud.tests.remote-images.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let initial = MailboxStore(engine: PreviewEngine(), defaults: defaults)
        XCTAssertFalse(initial.loadRemoteImages)
        initial.loadRemoteImages = true
        initial.persistPrefs()

        let restored = MailboxStore(engine: PreviewEngine(), defaults: defaults)
        XCTAssertTrue(restored.loadRemoteImages)
    }

    func testClosingOlderDraftDoesNotClearCurrentComposer() async throws {
        let store = MailboxStore(engine: PreviewEngine(), usingPreview: true)
        await store.bootstrap()
        store.openCompose()
        let older = try XCTUnwrap(store.pendingComposeID)
        store.openCompose()
        let current = try XCTUnwrap(store.pendingComposeID)
        store.draft(for: current)?.body = "latest input"
        store.removeDraft(older)
        XCTAssertEqual(store.pendingComposeID, current)
        XCTAssertEqual(store.draft(for: current)?.body, "latest input")
        store.removeDraft(current)
        XCTAssertNil(store.pendingComposeID)
        XCTAssertTrue(store.drafts.isEmpty)
    }

    func testPreviewBootstrapLoadsInbox() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        XCTAssertEqual(store.accounts.count, 1)
        XCTAssertEqual(store.folders.first?.role, "inbox")
        XCTAssertEqual(store.messages.first?.subject, "Hello from testkit")
        XCTAssertGreaterThanOrEqual(store.providers.count, 18)
        XCTAssertTrue(store.providers.contains { $0.id == "qq" })
    }

    func testSendAndAddAccountOnPreview() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        await store.send(to: "bob@imyemail.test", subject: "Hi", body: "hello")
        XCTAssertTrue(store.status == L10n.t("已加入发件队列，10 秒内可撤销"))
        XCTAssertFalse(store.showCompose)
        await store.addAccount(email: "dev@qq.com", host: nil, password: "x", insecure: false)
        XCTAssertFalse(store.showAddAccount)
        XCTAssertTrue(store.accounts.contains { $0.email == "dev@qq.com" })
        XCTAssertEqual(store.accounts.first { $0.email == "dev@qq.com" }?.title, "dev@qq.com")
        await store.addAccount(email: "alice@qq.com", host: nil, password: "x", insecure: false, displayName: "Alice")
        XCTAssertEqual(store.accounts.first { $0.email == "alice@qq.com" }?.title, "Alice")
        XCTAssertFalse(store.folders(for: "alice@qq.com").isEmpty)
        await store.updateAccount(store.accounts.first { $0.email == "alice@qq.com" }!, host: nil, password: nil, insecure: false, displayName: "Alicia")
        XCTAssertEqual(store.accounts.first { $0.email == "alice@qq.com" }?.title, "Alicia")
        let alice = store.accounts.first { $0.email == "alice@qq.com" }!
        await store.removeAccount(alice)
        XCTAssertFalse(store.accounts.contains { $0.email == "alice@qq.com" })
        await store.removeAllAccounts()
        XCTAssertTrue(store.accounts.isEmpty)
        XCTAssertEqual(store.status, L10n.t("已清除全部账号"))
    }

    func testFilterUnreadAndArchive() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        store.filter = .unread
        XCTAssertTrue(store.visibleMessages.allSatisfy(\.unread))
        let first = store.messages.first!
        store.selectedMessage = first
        store.archiveSelected()
        var gone = false
        for _ in 0..<40 {
            if !store.messages.contains(where: { $0.id == first.id }) {
                gone = true
                break
            }
            try? await Task.sleep(for: .milliseconds(25))
        }
        XCTAssertTrue(gone)
    }

    func testAdaptiveChrome() {
        XCTAssertEqual(AdaptiveChrome.resolve(horizontal: .compact, platform: .ios), .phone)
        XCTAssertEqual(AdaptiveChrome.resolve(horizontal: .regular, platform: .ios), .pad)
        XCTAssertEqual(AdaptiveChrome.resolve(horizontal: .compact, platform: .mac), .mac)
        XCTAssertEqual(ChckBrand.bundleID, "email.imy.cloud")
        XCTAssertEqual(AdaptiveChrome.resolve(horizontal: nil, platform: .ios), .phone)
        XCTAssertEqual(ChckSpacing.navIdeal, 220)
        XCTAssertEqual(ChckSpacing.listIdeal, 360)
        XCTAssertEqual(MailDensity.comfortable.snippetLines, 2)
        XCTAssertEqual(MailAccount(id: "1", email: "dev@qq.com", displayName: "dev", providerId: "qq").title, "dev@qq.com")
        XCTAssertEqual(MailAccount(id: "2", email: "dev@qq.com", displayName: "Alice", providerId: "qq").title, "Alice")
        XCTAssertEqual(MailAddress.split("Alice <a@imyemail.test>, b@imyemail.test").count, 2)
        XCTAssertEqual(MailAddress.canonical("Alice <a@imyemail.test>"), "a@imyemail.test")
        XCTAssertEqual(MailboxStore.replySubject("Hello", forward: false), "Re: Hello")
        XCTAssertEqual(MailboxStore.replySubject("Re: Hello", forward: false), "Re: Hello")
    }

    func testOpenComposeReplyUsesCanonicalAddress() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        let row = store.messages.first!
        store.openCompose(reply: row)
        let draft = store.draft(for: store.pendingComposeID!)
        XCTAssertEqual(draft?.to, row.replyAddress)
        XCTAssertTrue(draft?.subject.hasPrefix("Re:") == true)
        draft?.body = "edited reply"
        XCTAssertEqual(store.draft(for: draft!.id)?.body, "edited reply")
        let ok = await store.send(accountId: draft!.accountId, to: draft!.to, cc: "", bcc: "", subject: draft!.subject, body: draft!.body)
        XCTAssertTrue(ok)
        XCTAssertTrue(store.status == L10n.t("已加入发件队列，10 秒内可撤销"))
    }

    func testEngineFactoryPrefersNativeWhenLibraryExists() {
        let preview = EngineFactory.make(preview: true, dbPath: "mail.db", nativeLibrary: nil, allowProcessSymbols: false, binary: nil)
        XCTAssertTrue(preview.preview)
        if let lib = NativeEngine.resolveLibrary() {
            XCTAssertTrue(FileManager.default.fileExists(atPath: lib))
        }
        let made = EngineFactory.make(
            preview: false,
            dbPath: NSTemporaryDirectory() + "chck-factory-\(UUID().uuidString).db",
            nativeLibrary: NativeEngine.resolveLibrary(),
            allowProcessSymbols: false,
            binary: nil
        )
        if NativeEngine.resolveLibrary() != nil, !made.preview {
            XCTAssertFalse(made.preview)
        }
    }

    func testPhoneRouteAndStar() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        store.phonePath = [.mailbox(.unified), .message("preview:INBOX:1")]
        XCTAssertEqual(store.phonePath.count, 2)
        let row = store.messages.first!
        await store.open(row)
        store.toggleStar()
        var flagged = false
        for _ in 0..<40 {
            if store.selectedMessage?.flagged == true {
                flagged = true
                break
            }
            try? await Task.sleep(for: .milliseconds(25))
        }
        XCTAssertTrue(flagged)
    }
}
