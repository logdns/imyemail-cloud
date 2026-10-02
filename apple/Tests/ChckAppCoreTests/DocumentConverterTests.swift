import ChckAppCore
import XCTest

final class DocumentConverterTests: XCTestCase {
    func testConverterRequiresSecureOriginAndRejectsEmbeddedCredentials() throws {
        XCTAssertEqual(try DocumentConverter.endpoint("https://convert.example.test").host, "convert.example.test")
        XCTAssertEqual(try DocumentConverter.endpoint("http://127.0.0.1:8765").port, 8765)
        for value in ["http://convert.example.test", "file:///tmp/test", "https://user:secret@example.test", "https://example.test?token=secret", "https://example.test/redirect", "https://example.test#fragment"] {
            XCTAssertThrowsError(try DocumentConverter.endpoint(value), value)
        }
    }

    @MainActor
    func testDraftHTMLMatchesNativeMarkdownPreview() {
        let draft = ComposeDraft(accountId: "account", body: "# Native\n\n**你好** 👋")
        XCTAssertEqual(draft.outgoingHTML, MarkdownBody.render(draft.body))
        XCTAssertTrue(draft.outgoingHTML?.contains("<strong>你好</strong>") == true)
        draft.body = "<script>alert(1)</script>"
        XCTAssertFalse(draft.outgoingHTML?.contains("<script>") == true)
    }

    func testUnsupportedFileRejectedBeforeAnyUpload() async {
        do {
            _ = try await DocumentConverter.convert(file: URL(fileURLWithPath: "/tmp/document.zip"),
                endpoint: "https://example.invalid", token: "test")
            XCTFail("ZIP must not be uploaded")
        } catch DocumentConversionError.unsupportedFile {} catch { XCTFail("Unexpected: \(error)") }
    }

    func testRealMarkItDownServiceAndAuthenticationFailure() async throws {
        guard let endpoint = ProcessInfo.processInfo.environment["IMYEMAIL_CLOUD_TEST_CONVERTER_URL"] else {
            throw XCTSkip("Run against the isolated local MarkItDown service")
        }
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("chck-import-\(UUID().uuidString).html")
        try Data("<h1>Imported document</h1><p><strong>Preserve formatting</strong></p>".utf8).write(to: file)
        defer { try? FileManager.default.removeItem(at: file) }
        let markdown = try await DocumentConverter.convert(file: file, endpoint: endpoint,
            token: "chck-local-converter-tests-only-token")
        XCTAssertTrue(markdown.contains("# Imported document"))
        XCTAssertTrue(MarkdownBody.render(markdown).contains("<strong>Preserve formatting</strong>"))
        do {
            _ = try await DocumentConverter.convert(file: file, endpoint: endpoint, token: "invalid")
            XCTFail("Invalid credentials must not succeed")
        } catch DocumentConversionError.server(401) {} catch { XCTFail("Unexpected: \(error)") }
    }
}
