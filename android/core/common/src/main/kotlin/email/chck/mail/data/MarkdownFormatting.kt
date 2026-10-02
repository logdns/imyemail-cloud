package email.imy.cloud.data

enum class MarkdownFormat { Bold, Italic, Heading, Bullets, Numbered, Quote, Code, Link }
data class MarkdownEdit(val text: String, val start: Int, val end: Int)

object MarkdownFormatting {
    fun apply(text: String, selectionStart: Int, selectionEnd: Int, format: MarkdownFormat): MarkdownEdit {
        val start = minOf(selectionStart, selectionEnd).coerceIn(0, text.length)
        val end = maxOf(selectionStart, selectionEnd).coerceIn(start, text.length)
        val selected = text.substring(start, end)
        fun wrap(prefix: String, suffix: String, placeholder: String): MarkdownEdit {
            val content = selected.ifEmpty { placeholder }
            return MarkdownEdit(text.replaceRange(start, end, prefix + content + suffix), start + prefix.length, start + prefix.length + content.length)
        }
        return when (format) {
            MarkdownFormat.Bold -> wrap("**", "**", L10n.t("粗体"))
            MarkdownFormat.Italic -> wrap("*", "*", L10n.t("斜体"))
            MarkdownFormat.Link -> {
                val label = selected.ifEmpty { L10n.t("链接文字") }
                val url = "https://"
                MarkdownEdit(text.replaceRange(start, end, "[$label]($url)"), start + label.length + 3, start + label.length + 3 + url.length)
            }
            MarkdownFormat.Code -> {
                val content = selected.ifEmpty { L10n.t("代码") }
                val fence = "`".repeat(maxOf(3, (Regex("`+").findAll(content).maxOfOrNull { it.value.length } ?: 0) + 1))
                wrap("\n$fence\n", "\n$fence\n", L10n.t("代码"))
            }
            else -> {
                val lineStart = text.lastIndexOf('\n', start - 1).let { if (start == 0) 0 else it + 1 }
                val effectiveEnd = if (end > start && text[end - 1] == '\n') end - 1 else end
                val lineEnd = text.indexOf('\n', effectiveEnd).let { if (it < 0) text.length else it }
                val lines = text.substring(lineStart, lineEnd).split('\n')
                val replacement = lines.mapIndexed { index, line ->
                    val prefix = when (format) {
                        MarkdownFormat.Heading -> "## "
                        MarkdownFormat.Bullets -> "- "
                        MarkdownFormat.Numbered -> "${index + 1}. "
                        else -> "> "
                    }
                    prefix + line
                }.joinToString("\n")
                MarkdownEdit(text.replaceRange(lineStart, lineEnd, replacement), lineStart, lineStart + replacement.length)
            }
        }
    }
}
