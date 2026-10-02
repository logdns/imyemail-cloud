package email.imy.cloud

import android.database.sqlite.SQLiteDatabase
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import email.imy.cloud.chckcore.NativeBridge
import email.imy.cloud.data.JsonMailEngine
import email.imy.cloud.data.MarkdownBody
import java.io.File
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MarkdownNativeQueueTest {
    @Test fun installedJniPersistsTextAndHtmlInIsolatedQueue() = runBlocking {
        assertTrue(NativeBridge.available())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.cacheDir, "markdown-queue-test-${System.nanoTime()}").apply { mkdirs() }
        val db = File(directory, "mail.db")
        val handle = NativeBridge.nativeOpen(db.path)
        assertTrue(handle != 0L)
        try {
            val accountRequest = JSONObject().put("email", "qa@imyemail.test").put("imap_host", "127.0.0.1").put("smtp_host", "127.0.0.1")
            val accountResult = NativeBridge.nativeCall(handle, "add_account", JSONObject().put("json", accountRequest.toString()).toString())
            val account = JSONObject(accountResult)
            assertFalse(accountResult, account.has("error"))
            val engine = JsonMailEngine { method, args -> NativeBridge.nativeCall(handle, method, args) }
            val source = "# 中文排版\n\n**最终内容** 👋"
            val html = MarkdownBody.render(source)
            // Queue only: undo_window_secs is nonzero; no flush or worker opens this isolated DB.
            engine.send(account.getString("id"), "recipient@imyemail.test", "markdown-queue", source, html)
            SQLiteDatabase.openDatabase(db.path, null, SQLiteDatabase.OPEN_READONLY).use { database ->
                database.rawQuery("SELECT draft_json FROM outbox", null).use { rows ->
                    assertTrue(rows.moveToFirst())
                    val payload = JSONObject(rows.getString(0))
                    assertEquals(source, payload.getString("body_text"))
                    assertEquals(html, payload.getString("body_html"))
                }
            }
        } finally {
            NativeBridge.nativeClose(handle)
            directory.deleteRecursively()
        }
    }
}
