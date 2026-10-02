package email.imy.cloud.data

import org.commonmark.parser.Parser
import org.commonmark.renderer.html.HtmlRenderer
import org.commonmark.renderer.html.UrlSanitizer
import org.commonmark.node.*
import org.commonmark.ext.gfm.tables.TablesExtension
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension

object MarkdownBody {
    const val MAX_LENGTH = 200_000
    fun render(source: String): String {
        require(source.length <= MAX_LENGTH) { L10n.t("正文超过 20 万字符，请缩短后重试") }
        val extensions = listOf(TablesExtension.create(), StrikethroughExtension.create())
        val document = Parser.builder().extensions(extensions).build().parse(source)
        document.accept(object : AbstractVisitor() {
            override fun visit(image: Image) {
                // Keep alt text only; never turn imported images into network requests.
                var child = image.firstChild
                while (child != null) {
                    val next = child.next
                    child.unlink()
                    image.insertBefore(child)
                    child = next
                }
                image.unlink()
            }
        })
        val renderer = HtmlRenderer.builder().extensions(extensions).escapeHtml(true)
            .softbreak("<br />\n").sanitizeUrls(true).urlSanitizer(object : UrlSanitizer {
                override fun sanitizeLinkUrl(url: String): String = if (
                    !url.any { it.isISOControl() } &&
                    Regex("^(https?://|mailto:)", RegexOption.IGNORE_CASE).containsMatchIn(url)
                ) url else ""
                override fun sanitizeImageUrl(url: String) = ""
            }).build()
        return renderer.render(document)
    }
}
