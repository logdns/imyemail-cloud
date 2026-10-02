package email.imy.cloud

import email.imy.cloud.data.L10n

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import com.chaquo.python.Python
import com.chaquo.python.android.AndroidPlatform
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

object DocumentImporter {
    private const val MAX_BYTES = 8 * 1024 * 1024
    private val pythonLock = Any()

    suspend fun convert(context: Context, uri: Uri): String = withContext(Dispatchers.IO) {
        require(uri.scheme == "content") { L10n.t("请从文件选择器选择文档") }
        val resolver = context.contentResolver
        val name = resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
            if (it.moveToFirst()) it.getString(0) else null
        }.orEmpty()
        val extension = when {
            name.endsWith(".docx", true) -> ".docx"
            name.endsWith(".pdf", true) -> ".pdf"
            resolver.getType(uri) == "application/pdf" -> ".pdf"
            resolver.getType(uri) == "application/vnd.openxmlformats-officedocument.wordprocessingml.document" -> ".docx"
            else -> error(L10n.t("仅支持 Word .docx 和 PDF，旧版 .doc 请先另存为 .docx"))
        }
        val bytes = resolver.openInputStream(uri)?.use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                require(output.size() + count <= MAX_BYTES) { L10n.t("文件超过 8 MB，请选择较小的文档") }
                output.write(buffer, 0, count)
            }
            output.toByteArray()
        } ?: error(L10n.t("无法读取所选文件，请重新选择"))
        synchronized(pythonLock) {
            if (!Python.isStarted()) Python.start(AndroidPlatform(context))
            val output = Python.getInstance().getModule("document_import").callAttr("convert", bytes, extension).toString()
            val result = JSONObject(output)
            if (result.has("error")) error(result.getString("error"))
            result.getString("markdown")
        }
    }
}
