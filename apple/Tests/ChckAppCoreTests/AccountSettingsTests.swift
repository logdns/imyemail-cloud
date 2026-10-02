@testable import ChckAppCore
import XCTest

final class AccountSettingsTests: XCTestCase {
    func testInvalidPortsAndHostsCannotBeSilentlyDefaulted() {
        var settings = AccountSettings()
        settings.email = "me@example.test"
        settings.imapHost = "imap.example.test"
        settings.smtpHost = "smtp.example.test"
        XCTAssertNil(settings.validationError)
        settings.smtpPort = 0
        XCTAssertNotNil(settings.validationError)
        settings.smtpPort = 465
        settings.smtpHost = "https://smtp.example.test"
        XCTAssertNotNil(settings.validationError)
    }

    func testNativeAndCLISettingsRoundTrip() async throws {
        #if os(macOS)
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("chck-settings-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        guard let library = NativeEngine.resolveLibrary(), let binary = CliEngine.resolveBinary() else {
            throw XCTSkip("Build core or set IMYEMAIL_CLOUD_MAIL_LIB and IMYEMAIL_CLOUD_MAIL for the native integration tests")
        }
        let native = try NativeEngine(dbPath: folder.appendingPathComponent("native.db").path, libraryPath: library)
        let cli = CliEngine(binary: binary, dbPath: folder.appendingPathComponent("cli.db").path)
        for engine: any MailEngineClient in [native, cli] {
            var settings = AccountSettings()
            settings.email = "me@outlook.com"
            settings.displayName = "Custom"
            settings.imapHost = "custom.example.test"
            settings.imapPort = 1993
            settings.smtpHost = "relay.example.test"
            settings.smtpPort = 1465
            settings.username = "custom-login"
            settings.smtpStarttls = true
            settings.acceptInvalidCerts = true
            let account = try await engine.saveAccountSettings(settings, id: nil, password: "")
            let loaded = try await engine.accountSettings(id: account.id)
            XCTAssertEqual(loaded, settings)
            settings.smtpStarttls = false
            settings.acceptInvalidCerts = false
            _ = try await engine.saveAccountSettings(settings, id: account.id, password: "")
            let updated = try await engine.accountSettings(id: account.id)
            XCTAssertEqual(updated, settings)
            try await engine.removeAccount(id: account.id)
        }
        #endif
    }

    @MainActor
    func testReplyDoesNotQuoteAnotherSelectedMessagesBody() async {
        let store = MailboxStore(engine: PreviewEngine())
        await store.bootstrap()
        let row = store.messages.first!
        store.body = MailBody(text: "unrelated private message", html: "", id: "different-message")
        store.openCompose(reply: row)
        let draft = store.draft(for: store.pendingComposeID!)!
        XCTAssertFalse(draft.body.contains("unrelated private message"))
        XCTAssertTrue(draft.body.contains(row.snippet))
    }
}
