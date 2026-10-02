@testable import ChckAppCore
import XCTest

final class MarkdownBodyTests: XCTestCase {
    func testMailFormattingAndTables() {
        let html = MarkdownBody.render("""
        # 周报

        **完成** 与 *计划*，~~取消~~，`code`

        - 第一项
        - [x] 第二项

        > 引用

        ```swift
        let x = "<tag>"
        ```

        | 项目 | 状态 |
        | --- | --- |
        | 编辑器 | 完成 |
        """)
        for tag in ["<h1>周报</h1>", "<strong>完成</strong>", "<em>计划</em>", "<del>取消</del>", "<ul>", "☑", "<blockquote", "&lt;tag&gt;", "<table", "<th ", "<td "] {
            XCTAssertTrue(html.contains(tag), "Missing \(tag): \(html)")
        }
    }

    func testRawHTMLAndUnsafeURLsCannotExecuteOrLoadResources() {
        let html = MarkdownBody.render("""
        <script>alert(1)</script>

        <img src="https://tracker.test/pixel" onerror="alert(1)">

        [bad](javascript:alert%281%29) [file](file:///etc/passwd)
        [data](data:text/html,hello) [good](https://example.com?q=1&b=2)
        ![tracking](https://tracker.test/pixel)
        ![local](file:///etc/passwd)
        """)
        for unsafe in ["<script", "<img", "href=\"javascript:", "href=\"file:", "href=\"data:"] {
            XCTAssertFalse(html.contains(unsafe), html)
        }
        XCTAssertTrue(html.contains("&lt;script&gt;"))
        XCTAssertTrue(html.contains("href=\"https://example.com?q=1&amp;b=2\""))
        XCTAssertTrue(html.contains("href=\"https://tracker.test/pixel\""))
    }

    func testFormattingUsesUTF16SelectionWithoutLosingSurroundingText() {
        let source = "👋 你好 world"
        let edit = MarkdownFormatting.bold.edit(source, selection: (source as NSString).range(of: "你好"))
        let result = (source as NSString).replacingCharacters(in: edit.range, with: edit.replacement)
        XCTAssertEqual(result, "👋 **你好** world")
        XCTAssertEqual((result as NSString).substring(with: edit.selection), "你好")
    }

    func testMultilineListDoesNotModifyNextUnselectedLine() {
        let source = "one\ntwo\nthree"
        let edit = MarkdownFormatting.numbered.edit(source, selection: NSRange(location: 0, length: 8))
        XCTAssertEqual((source as NSString).replacingCharacters(in: edit.range, with: edit.replacement), "1. one\n2. two\nthree")
    }

    func testLinkSelectsURLAndCodeHandlesBackticks() {
        let edit = MarkdownFormatting.link.edit("文档", selection: NSRange(location: 0, length: 2))
        XCTAssertEqual((edit.replacement as NSString).substring(with: edit.selection), "https://example.com")
        let code = "a `code` b"
        let wrapped = MarkdownFormatting.code.edit(code, selection: NSRange(location: 0, length: (code as NSString).length))
        XCTAssertTrue(MarkdownBody.render(wrapped.replacement).contains("<code>a `code` b</code>"))
    }

    func testPayloadPreservesMarkdownSourceHTMLAndEnvelope() {
        let source = "## 标题\n\n**你好** 👋"
        let html = MarkdownBody.render(source)
        let payload = NativeEngine.sendPayload(accountId: "account-2", to: "to@example.test", cc: "cc@example.test", bcc: "bcc@example.test", subject: "测试", body: source, html: html, undo: 10)
        XCTAssertEqual(payload["body_text"] as? String, source)
        XCTAssertEqual(payload["body_html"] as? String, html)
        XCTAssertEqual(payload["account_id"] as? String, "account-2")
        XCTAssertEqual(payload["bcc"] as? [String], ["bcc@example.test"])
        XCTAssertEqual(payload["undo_window_secs"] as? Int, 10)
        let plain = NativeEngine.sendPayload(accountId: "account-2", to: "", cc: "", bcc: "", subject: "", body: source, html: nil, undo: nil)
        XCTAssertNil(plain["body_html"])
    }
    #if os(macOS)
    func testNativeAndCLIPersistDraftAndQueueSend() async throws {
        guard let library = NativeEngine.resolveLibrary(), let binary = CliEngine.resolveBinary() else {
            throw XCTSkip("Build native core and CLI first")
        }
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("chck-markdown-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        for native in [true, false] {
            let db = folder.appendingPathComponent(native ? "native.db" : "cli.db").path
            let engine: any MailEngineClient
            if native {
                engine = try NativeEngine(dbPath: db, libraryPath: library, fileSecrets: true)
            } else {
                engine = CliEngine(binary: binary, dbPath: db, fileSecrets: true)
            }
            var settings = AccountSettings()
            settings.email = "markdown@example.test"
            settings.imapHost = "imap.example.test"
            settings.smtpHost = "smtp.example.test"
            let account = try await engine.saveAccountSettings(settings, id: nil, password: "")
            let source = "# 草稿\n\n**中文** 👋\n\n> 引用"
            let html = MarkdownBody.render(source)
            let draft = try await engine.saveDraft(accountId: account.id, to: "reader@example.test", cc: "", bcc: "private@example.test", subject: "Markdown", body: source, html: html)
            let queued = try await engine.send(accountId: account.id, to: "reader@example.test", cc: "", bcc: "private@example.test", subject: "Markdown", body: source, html: html)
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/sqlite3")
            process.arguments = ["-json", db, "SELECT id, draft_json, state FROM outbox ORDER BY id"]
            let pipe = Pipe()
            process.standardOutput = pipe
            try process.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()
            XCTAssertEqual(process.terminationStatus, 0)
            let rows = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [[String: Any]])
            XCTAssertEqual(Set(rows.compactMap { $0["id"] as? String }), Set([draft, queued]))
            for row in rows {
                let json = try XCTUnwrap(row["draft_json"] as? String)
                let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any])
                XCTAssertEqual(payload["body_text"] as? String, source)
                XCTAssertEqual(payload["body_html"] as? String, html)
                XCTAssertEqual(payload["bcc"] as? [String], ["private@example.test"])
                XCTAssertEqual(row["state"] as? String, row["id"] as? String == draft ? "draft" : "queued")
            }
        }
    }
    #endif

}
