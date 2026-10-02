package email.imy.cloud.data
import kotlin.test.*

class MarkdownFormattingTest {
    @Test fun formatsUnicodeSelectionWithoutLosingSurroundingText() {
        val result = MarkdownFormatting.apply("你好 👋 世界", 3, 5, MarkdownFormat.Bold)
        assertEquals("你好 **👋** 世界", result.text)
        assertEquals("👋", result.text.substring(result.start, result.end))
    }
    @Test fun reversedSelectionAndMultilineLists() {
        val result = MarkdownFormatting.apply("甲\n乙\n丙", 4, 0, MarkdownFormat.Numbered)
        assertEquals("1. 甲\n2. 乙\n丙", result.text)
    }
    @Test fun linkSelectsDestination() {
        val result = MarkdownFormatting.apply("链接", 0, 2, MarkdownFormat.Link)
        assertEquals("[链接](https://)", result.text)
        assertEquals("https://", result.text.substring(result.start, result.end))
    }
    @Test fun codeWithBackticksUsesLongerFence() {
        val result = MarkdownFormatting.apply("```code```", 0, 10, MarkdownFormat.Code)
        assertTrue(result.text.startsWith("\n````\n"))
        assertTrue("```code```" in MarkdownBody.render(result.text))
    }
}
