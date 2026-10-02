package email.imy.cloud

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.TextRange
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@OptIn(ExperimentalTestApi::class)
@RunWith(AndroidJUnit4::class)
class MarkdownEditorTest {
    @get:Rule val compose = createAndroidComposeRule<DesignPreviewActivity>()
    private fun capture(name: String) {
        val instrumentation = androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        val file = java.io.File(instrumentation.targetContext.getExternalFilesDir(null), "$name.png")
        instrumentation.uiAutomation.takeScreenshot().let { bitmap ->
            file.outputStream().use { bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
            bitmap.recycle()
        }
    }
    @Test fun nativeSelectionFormattingAndPreviewKeepSource() {
        try {
            compose.waitUntil(15000) { compose.onAllNodesWithText("写邮件", useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty() }
        } catch (error: Exception) {
            capture("markdown-test-failure")
            android.util.Log.e("MarkdownTest", compose.onRoot(useUnmergedTree = true).printToString())
            throw error
        }
        compose.onNodeWithText("写邮件", useUnmergedTree = true).performClick()
        compose.onNodeWithTag("compose-body").performScrollTo().performTextInput("你好 👋 世界")
        compose.onNodeWithTag("compose-body").performTextInputSelection(TextRange(3, 5))
        compose.onNodeWithContentDescription("粗体").performScrollTo().performClick()
        compose.onNodeWithTag("compose-body").assertTextContains("你好 **👋** 世界")
        compose.onNodeWithText("预览", useUnmergedTree = true).performScrollTo().performClick()
        compose.onNodeWithText("编辑", useUnmergedTree = true).performScrollTo().performClick()
        compose.onNodeWithTag("compose-body").assertTextContains("你好 **👋** 世界")
        compose.onNodeWithTag("compose-body").performTextReplacement("# 项目进展\n\n你好，**本周更新** 已整理完成。\n\n- 原生 Markdown 编辑\n- Word / PDF 导入正文\n\n> 期待你的反馈。")
        compose.runOnUiThread {
            val manager = compose.activity.getSystemService(android.content.Context.INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            manager.hideSoftInputFromWindow(compose.activity.window.decorView.windowToken, 0)
        }
        compose.waitForIdle()
        Thread.sleep(700)
        capture("markdown-editor")
        compose.onNodeWithText("预览", useUnmergedTree = true).performScrollTo().performClick()
        compose.waitForIdle()
        compose.waitUntil(10000) { compose.onAllNodesWithTag("markdown-preview").fetchSemanticsNodes().isNotEmpty() }
        compose.waitForIdle()
        Thread.sleep(1000)
        capture("markdown-preview")
    }
}
