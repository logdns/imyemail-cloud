import XCTest
import ChckAppCore

@MainActor
final class MailClientUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .portrait
        app = XCUIApplication()
        app.launchArguments = ["-chck.language", "zh-CN", "-chck.notify", "NO", "-AppleLanguages", "(zh-Hans)", "-AppleLocale", "zh_CN"]
    }

    override func tearDownWithError() throws {
        app.terminate()
        XCUIDevice.shared.orientation = .portrait
    }

    private func launch(preview: Bool = true) {
        app.launchEnvironment["IMYEMAIL_CLOUD_PREVIEW"] = preview ? "1" : "0"
        app.launch()
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let deny = springboard.alerts.buttons["不允许"]
        if deny.exists { deny.tap() }
    }

    private func capture(_ name: String) {
        // Core Animation can still be rotating after the accessibility tree settles.
        Thread.sleep(forTimeInterval: 1)
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func openInbox() {
        if !app.staticTexts["Hello from testkit"].waitForExistence(timeout: 3) {
            app.buttons["统一收件箱"].firstMatch.tap()
        }
        XCTAssertTrue(app.staticTexts["Hello from testkit"].waitForExistence(timeout: 10))
    }

    private func openCompose() {
        let phoneButton = app.buttons["写信"].firstMatch
        if phoneButton.exists { phoneButton.tap() }
        else { app.buttons["新邮件"].firstMatch.tap() }
        XCTAssertTrue(app.textFields["compose.to"].waitForExistence(timeout: 5))
    }

    private func revealMarkdownMode() {
        // A Form cell containing the editor can be taller than the keyboard viewport.
        // Scroll its outer gutter rather than the text view's internal scroll area.
        for _ in 0..<8 {
            let frame = app.segmentedControls["compose.mode"].frame
            if frame.minY > 145 && frame.maxY < app.frame.height * 0.55 { return }
            let startY: CGFloat = frame.minY < 145 ? 0.30 : 0.48
            let endY: CGFloat = frame.minY < 145 ? 0.48 : 0.30
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.97, dy: startY))
                .press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.97, dy: endY)))
        }
    }

    func testSixLanguageSettingsAndAppearance() {
        let cases = [("en", "Cancel", "Settings", "English"), ("zh-TW", "取消", "設定", "繁體中文"), ("zh-CN", "取消", "设置", "简体中文"), ("ja", "キャンセル", "設定", "日本語"), ("fr", "Annuler", "Paramètres", "Français"), ("es", "Cancelar", "Configuración", "Español")]
        for (code, cancel, settingsName, nativeName) in cases {
            app.launchArguments = ["-chck.language", code, "-chck.notify", "NO", "-chck.appearance", code == "en" ? "light" : "dark"]
            launch(preview: false)
            if app.buttons[cancel].waitForExistence(timeout: 5) { app.buttons[cancel].firstMatch.tap() }
            let settings = app.buttons.matching(identifier: settingsName).allElementsBoundByIndex.first { $0.isHittable }
            XCTAssertNotNil(settings, code)
            settings?.tap()
            XCTAssertTrue(app.staticTexts[nativeName].firstMatch.waitForExistence(timeout: 5) || app.buttons.containing(.staticText, identifier: nativeName).firstMatch.exists, code)
            capture("brand-settings-" + code)
            app.terminate()
        }
    }

    func testNativeOnboardingLoadsProviderCatalogue() {
        launch(preview: false)
        XCTAssertTrue(app.buttons["自定义 IMAP/SMTP"].waitForExistence(timeout: 15))
        // Providers come through the real, statically linked Rust FFI.
        XCTAssertTrue(app.buttons.containing(.staticText, identifier: "Gmail").firstMatch.exists)
        XCTAssertFalse(app.staticTexts["演示模式 · 示例邮件"].exists)
        XCTAssertFalse(app.staticTexts["邮件服务未能启动，请重新打开或重新安装应用。"].exists)
        capture("native-onboarding")
    }

    func testReadAndRotate() {
        launch()
        openInbox()
        capture("preview-inbox-portrait")
        app.staticTexts["Hello from testkit"].firstMatch.tap()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 10))
        capture("preview-reading-portrait")
        XCUIDevice.shared.orientation = .landscapeLeft
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 10))
        capture("preview-reading-landscape")
        XCUIDevice.shared.orientation = .portrait
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 10))
    }

    func testComposeFitsScreenAndProtectsDraft() {
        launch()
        openCompose()
        let recipient = app.textFields["compose.to"]
        let subject = app.textFields["compose.subject"]
        XCTAssertGreaterThanOrEqual(recipient.frame.minX, app.frame.minX)
        XCTAssertLessThanOrEqual(recipient.frame.maxX, app.frame.maxX)
        recipient.tap()
        recipient.typeText("alice@imyemail.test")
        subject.tap()
        subject.typeText("iOS draft regression")
        let body = app.textViews["compose.body"]
        body.tap()
        body.typeText("Keep my latest input")
        XCTAssertEqual(app.buttons.matching(identifier: "compose.send").count, 1)
        XCTAssertFalse(app.buttons["发送邮件"].exists)
        capture("preview-compose")
        app.buttons["取消"].firstMatch.tap()
        app.buttons["继续编辑"].tap()
        XCTAssertEqual(subject.value as? String, "iOS draft regression")
        XCTAssertEqual(body.value as? String, "Keep my latest input")
        app.buttons["取消"].firstMatch.tap()
        app.buttons["删除草稿"].tap()
        XCTAssertTrue(recipient.waitForNonExistence(timeout: 5))
        openCompose()
        XCTAssertEqual(app.textFields["compose.subject"].value as? String, app.textFields["compose.subject"].placeholderValue ?? "")
    }

    func testLargeTextComposeAndSettings() {
        app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        launch()
        let settings = app.buttons.matching(identifier: "设置").allElementsBoundByIndex.first { $0.isHittable }
        XCTAssertNotNil(settings)
        settings?.tap()
        XCTAssertTrue(app.buttons["关闭"].waitForExistence(timeout: 5))
        capture("preview-settings-large-text")
        app.buttons["关闭"].tap()
        openCompose()
        let subject = app.textFields["compose.subject"]
        for _ in 0..<8 where !subject.isHittable {
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.44))
                .press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.30)))
        }
        XCTAssertTrue(subject.isHittable)
        XCTAssertLessThanOrEqual(subject.frame.maxX, app.frame.maxX)
        capture("preview-compose-large-text")
    }

    func testNativeMarkdownFormattingPreviewAndImportEntry() {
        launch()
        openCompose()
        let body = app.textViews["compose.body"]
        body.tap()
        body.typeText("# Native Markdown\n\n")
        app.buttons["markdown.bold"].tap()
        body.typeText("Formatted")
        XCTAssertTrue((body.value as? String ?? "").contains("**Formatted**"))
        let beforeUndo = body.value as? String
        app.buttons["markdown.undo"].tap()
        XCTAssertNotEqual(body.value as? String, beforeUndo)
        app.buttons["markdown.redo"].tap()
        XCTAssertEqual(body.value as? String, beforeUndo)
        app.buttons["keyboard.preview"].tap()
        capture("markdown-preview-switch")
        if !app.webViews.firstMatch.exists { app.swipeUp() }
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.webViews.staticTexts["Native Markdown"].waitForExistence(timeout: 5))
        capture("markdown-preview")
        app.buttons["编辑"].tap()
        XCTAssertTrue((body.value as? String ?? "").contains("**Formatted**"))
        capture("markdown-native-editor")
        app.buttons["compose.import"].tap()
        XCTAssertTrue(app.buttons["选择文件"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["转换服务设置"].exists)
        app.buttons["关闭"].tap()
        XCTAssertTrue((body.value as? String ?? "").contains("**Formatted**"))
    }

    func testSimulatorMarkItDownConversionWhenConfigured() async throws {
        guard let endpoint = ProcessInfo.processInfo.environment["IMYEMAIL_CLOUD_TEST_CONVERTER_URL"],
              endpoint.hasPrefix("http://127.0.0.1:") else {
            throw XCTSkip("Set IMYEMAIL_CLOUD_TEST_CONVERTER_URL to the isolated local conversion service")
        }
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("conversion-\(UUID().uuidString).html")
        try Data("<h1>Simulator import</h1><p><strong>Formatted body</strong></p>".utf8).write(to: file)
        defer { try? FileManager.default.removeItem(at: file) }
        let result = try await DocumentConverter.convert(file: file, endpoint: endpoint,
            token: "chck-local-converter-tests-only-token")
        XCTAssertTrue(result.contains("# Simulator import"))
        XCTAssertTrue(result.contains("**Formatted body**"))
        XCTAssertTrue(MarkdownBody.render(result).contains("<strong>Formatted body</strong>"))
    }
}
