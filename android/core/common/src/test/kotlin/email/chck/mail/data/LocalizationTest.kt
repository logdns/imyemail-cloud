package email.imy.cloud.data
import kotlin.test.*
class LocalizationTest {
    @Test fun englishFallbackAndSixCatalogs() {
        assertEquals("en", L10n.normalized(null))
        val expected = listOf("Settings", "設定", "设置", "設定", "Paramètres", "Configuración")
        L10n.codes.zip(expected).forEach { (code, text) -> assertEquals(text, L10n.t("设置", code)) }
        assertEquals("Settings", L10n.t("设置", "unknown"))
    }
    @Test fun preservesInterpolationAndUnknownContent() {
        assertEquals("XPH1X，second，third", L10n.t("XPH1X，second，third", "zh-CN"))
        assertTrue(L10n.t("已更新 42", "en").contains("42"))
        assertEquals("A customer subject 你好", L10n.t("A customer subject 你好", "en"))
    }
}
