package email.imy.cloud.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking

class PreviewEngineTest {
    @Test
    fun previewHasInbox() = runBlocking {
        val engine = PreviewEngine()
        assertEquals("dev@imyemail.test", engine.accounts().first().email)
        assertEquals("inbox", engine.folders("preview").first { it.role == "inbox" }.role)
        assertEquals("Hello from testkit", engine.messages("preview:INBOX").first().subject)
    }

    @Test
    fun providersAndAddAccountAndSend() = runBlocking {
        val engine = PreviewEngine()
        assertTrue(engine.providers().any { it.id == "qq" && it.imapHost == "imap.qq.com" })
        assertEquals(20, engine.providers().size)
        val acc = engine.addAccount("dev@qq.com", null, "auth-code", false)
        assertEquals("qq", acc.providerId)
        engine.send(acc.id, "bob@imyemail.test", "Hi", "hello")
        engine.sync("${acc.id}:INBOX")
    }

    @Test
    fun searchAndFlags() = runBlocking {
        val engine = PreviewEngine()
        assertEquals(1, engine.search("testkit").size)
        engine.setStarred("preview:INBOX:1", true)
        engine.setRead("preview:INBOX:1", true)
        val row = engine.messages("preview:INBOX").first()
        assertTrue(row.starred)
        assertTrue(!row.unread)
    }
}
