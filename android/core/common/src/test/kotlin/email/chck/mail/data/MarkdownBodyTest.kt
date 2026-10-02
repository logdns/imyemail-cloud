package email.imy.cloud.data

import kotlin.test.*

class MarkdownBodyTest {
    @Test fun rendersEmailFormattingAndUnicode() {
        val html = MarkdownBody.render("# 标题 👋\n\n**粗体** *斜体* ~~旧~~\n\n- 列表\n\n> 引用\n\n| 名称 | 状态 |\n| --- | --- |\n| 安卓 | 完成 |\n\n```\n<code>\n```")
        for (tag in listOf("<h1>", "<strong>", "<em>", "<del>", "<ul>", "<blockquote>", "<table>", "<pre><code>")) assertTrue(tag in html, html)
        assertTrue("标题 👋" in html)
        assertTrue("&lt;code&gt;" in html)
    }
    @Test fun escapesRawHtmlAndRemovesTrackingImages() {
        val html = MarkdownBody.render("<script>alert(1)</script>\n\n![说明](https://tracker.test/p.png) ![local](file:///secret)")
        assertFalse("<script>" in html)
        assertFalse("<img" in html)
        assertFalse("tracker.test" in html)
        assertTrue("说明" in html)
    }
    @Test fun rejectsDangerousLinksButKeepsSafeLinks() {
        for (url in listOf("javascript:alert%281%29", "data:text/html,evil", "file:///secret", "content://secret", "//tracker.test", "relative")) {
            val html = MarkdownBody.render("[click]($url)")
            assertTrue("href=\"\"" in html, html)
        }
        assertTrue("href=\"https://example.test\"" in MarkdownBody.render("[site](https://example.test)"))
        assertTrue("mailto:qa@example.test" in MarkdownBody.render("[mail](mailto:qa@example.test)"))
    }
    @Test fun boundsLargeInput() {
        assertFailsWith<IllegalArgumentException> { MarkdownBody.render("a".repeat(MarkdownBody.MAX_LENGTH + 1)) }
    }
    @Test fun preservesCodeAndSoftLineBreaks() {
        assertTrue("a<br />\nb" in MarkdownBody.render("a\nb"))
        assertTrue("**literal**" in MarkdownBody.render("`**literal**`"))
    }
}
