import XCTest
import ChckDesign
final class LocalizationTests: XCTestCase {
    func testDefaultAndUnknownLocale() {
        XCTAssertEqual(L10n.normalized(nil), "en")
        XCTAssertEqual(L10n.t("设置", language: "unknown"), "Settings")
    }
    func testSixOfflineCatalogs() {
        let expected = ["Settings", "設定", "设置", "設定", "Paramètres", "Configuración"]
        for (code, value) in zip(L10n.codes, expected) { XCTAssertEqual(L10n.t("设置", language: code), value) }
    }
    func testInterpolationAndUnknownContent() {
        XCTAssertEqual(L10n.t("XPH1X，second，third", language: "zh-CN"), "XPH1X，second，third")
        XCTAssertTrue(L10n.t("已更新 42", language: "en").contains("42"))
        XCTAssertEqual(L10n.t("A customer subject 你好", language: "en"), "A customer subject 你好")
    }
}
