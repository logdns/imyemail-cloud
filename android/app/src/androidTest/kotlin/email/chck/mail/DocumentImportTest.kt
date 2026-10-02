package email.imy.cloud

import androidx.core.content.FileProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import email.imy.cloud.data.MarkdownBody
import java.io.File
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DocumentImportTest {
    private suspend fun convert(name: String): String {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val target = instrumentation.targetContext
        val file = File(target.cacheDir, "import-test/$name")
        file.parentFile!!.mkdirs()
        instrumentation.context.assets.open(name).use { input -> file.outputStream().use { input.copyTo(it) } }
        return try { DocumentImporter.convert(target, FileProvider.getUriForFile(target, "email.imy.cloud.test.documents", file)) }
        finally { file.delete() }
    }
    @Test fun wordUsesRealMarkItDownAndPreservesFormatting() = runBlocking {
        val result = convert("sample.docx")
        assertTrue(result, result.contains("# 原生 Markdown 文档"))
        assertTrue(result, result.contains("**导入粗体**"))
        assertTrue(MarkdownBody.render(result).contains("<strong>导入粗体</strong>"))
    }
    @Test fun pdfUsesRealMarkItDownOnDevice() = runBlocking {
        assertTrue(convert("sample.pdf").contains("MarkItDown PDF import on Android"))
    }
    @Test fun emptyAndBrokenPdfsReportFailure() = runBlocking {
        for (name in listOf("empty.pdf", "broken.pdf")) {
            var failed = false
            try { convert(name) } catch (error: IllegalStateException) { failed = true }
            assertTrue("$name should fail visibly", failed)
        }
    }
}
