package email.imy.cloud.data

import java.util.Properties

/** Application copy only; never translate mailbox content or user input. */
object L10n {
    val codes = listOf("en", "zh-TW", "zh-CN", "ja", "fr", "es")
    val names = listOf("English", "繁體中文", "简体中文", "日本語", "Français", "Español")
    @Volatile var language: String = "en"
    fun normalized(code: String?) = code?.takeIf { it in codes } ?: "en"
    private val catalogs by lazy {
        codes.associateWith { code ->
            Properties().apply {
                L10n::class.java.classLoader?.getResourceAsStream("locales/$code.properties")?.reader(Charsets.UTF_8)?.use { load(it) }
            }.entries.associate { it.key.toString() to it.value.toString() }
        }
    }
    fun t(text: String, locale: String = language): String {
        val table = catalogs[normalized(locale)] ?: return text
        table[text]?.let { return it }
        for ((key, value) in table.entries.sortedByDescending { it.key.length }) {
            if (!key.contains("XPH")) continue
            val pattern = key.split(Regex("XPH[0-9]+X")).joinToString("([\\s\\S]*?)") { Regex.escape(it) }
            val match = Regex("^$pattern$").matchEntire(text) ?: continue
            return Regex("XPH([0-9]+)X").replace(value) { token ->
                match.groupValues.getOrNull(token.groupValues[1].toInt() + 1) ?: token.value
            }
        }
        return text
    }
}
