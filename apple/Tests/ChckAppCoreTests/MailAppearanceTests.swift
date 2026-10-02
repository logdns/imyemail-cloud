import ChckAppCore
import XCTest

final class MailAppearanceTests: XCTestCase {
    func testSystemDoesNotForceAColorScheme() {
        XCTAssertNil(MailAppearance.system.colorScheme)
        XCTAssertEqual(MailAppearance.light.colorScheme, .light)
        XCTAssertEqual(MailAppearance.dark.colorScheme, .dark)
    }

    func testHTMLUsesSelectedThemeWithoutWeakeningContentPolicy() {
        for theme in ["light", "dark"] {
            let html = MailContentPolicy.document(html: "<p>正文</p>", allowRemote: false, colorScheme: theme)
            XCTAssertTrue(html.contains("color-scheme: \(theme);"))
            XCTAssertTrue(html.contains("default-src 'none'"))
            XCTAssertTrue(html.contains("img-src data:"))
            XCTAssertTrue(html.contains("<p>正文</p>"))
        }
        let fallback = MailContentPolicy.document(html: "", allowRemote: false, colorScheme: "dark; background:url(https://invalid.test)")
        XCTAssertTrue(fallback.contains("color-scheme: light dark;"))
        XCTAssertFalse(fallback.contains("invalid.test"))
    }

    func testRemoteImageOptInOnlyAllowsHTTPSImages() {
        let blocked = MailContentPolicy.document(
            html: #"<img src="https://images.example.test/mail.png"><script>alert(1)</script>"#,
            allowRemote: false
        )
        XCTAssertTrue(blocked.contains("img-src data:"))
        XCTAssertFalse(blocked.contains("img-src https:"))

        let allowed = MailContentPolicy.document(
            html: #"<img src="https://images.example.test/mail.png">"#,
            allowRemote: true
        )
        XCTAssertTrue(allowed.contains("default-src 'none'"))
        XCTAssertTrue(allowed.contains("img-src https: data:"))
        XCTAssertTrue(allowed.contains("base-uri 'none'"))
        XCTAssertTrue(allowed.contains("form-action 'none'"))
        XCTAssertTrue(allowed.contains("frame-src 'none'"))
        XCTAssertFalse(allowed.contains("script-src"))
    }
}
