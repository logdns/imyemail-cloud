package email.imy.cloud.data

import kotlinx.serialization.json.*
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.test.assertFailsWith
import kotlinx.coroutines.runBlocking

class JsonMailEngineTest {
    @Test
    fun parsesAccountAndProviders() = runBlocking<Unit> {
        val engine = JsonMailEngine { method, _ ->
            when (method) {
                "list_providers" -> """[{"id":"qq","display_name_zh":"QQ 邮箱 / Foxmail","imap_host":"imap.qq.com","auth_kind":"authcode"}]"""
                "test_account" -> """{"steps":[{"kind":"auth","status":"ok"}]}"""
                "add_account" -> """{"id":"a1","email":"dev@qq.com","display_name":"Dev","provider_id":"qq"}"""
                "list_accounts" -> """[{"id":"a1","email":"dev@qq.com","display_name":"Dev","provider_id":"qq"}]"""
                "list_folders" -> """[{"id":"f1","path":"INBOX","role":"inbox","unread_count":1}]"""
                "list_messages" -> """[{"id":"m1","subject":"Hi","from":"alice@imyemail.test","snippet":"Hello","seen":false}]"""
                "body" -> """{"text":"Hello","html_sanitized":"<p>Hello</p>"}"""
                else -> "{}"
            }
        }
        assertEquals("imap.qq.com", engine.providers().first().imapHost)
        val acc = engine.addAccount("dev@qq.com", null, "x", false)
        assertEquals("qq", acc.providerId)
        assertEquals("inbox", engine.folders(acc.id).first().role)
        assertEquals("Hi", engine.messages("f1").first().subject)
        assertEquals("Hello", engine.body("m1").text)
    }

    @Test
    fun factoryCliArgsUseFfiMethod() = runBlocking<Unit> {
        var seen = emptyList<String>()
        val engine = JsonMailEngine { method, args ->
            seen = listOf("ffi", "--method", method, "--args", args)
            when (method) {
                "list_accounts" -> "[]"
                else -> "{}"
            }
        }
        engine.accounts()
        assertEquals("ffi", seen[0])
        assertEquals("list_accounts", seen[2])
    }

    @Test
    fun flushDueSendsReadsCount() = runBlocking<Unit> {
        val engine = JsonMailEngine { method, _ ->
            when (method) {
                "flush_due_sends" -> """{"flushed":2}"""
                else -> "{}"
            }
        }
        assertEquals(2, engine.flushDueSends())
    }

    @Test
    fun nativeErrorIsNeverTreatedAsEmptyInboxOrSuccessfulSend() = runBlocking<Unit> {
        val engine = JsonMailEngine { _, _ -> """{"error":"authentication failed"}""" }
        assertFailsWith<IllegalStateException> { engine.accounts() }
        assertFailsWith<IllegalStateException> { engine.send("a", "b@imyemail.test", "subject", "body") }
        assertFailsWith<IllegalStateException> { engine.delete("m") }
    }

    @Test
    fun failedConnectionProbeDoesNotSaveAccount() = runBlocking<Unit> {
        val calls = mutableListOf<String>()
        val engine = JsonMailEngine { method, _ ->
            calls += method
            """{"steps":[{"kind":"auth","status":"failed"},{"kind":"folders","status":"skipped"}]}"""
        }
        assertFailsWith<IllegalStateException> { engine.addAccount("a@imyemail.test", null, "x", false) }
        assertEquals(listOf("test_account"), calls)
    }

    @Test
    fun nestedMessagesAndEscapedBodiesMatchRustContract() {
        val rows = parseMessages("""[{"id":"a:INBOX:1","account_id":"a","from":[{"name":"A {name}","email":"a@imyemail.test"}],"subject":"Hi } { \"quoted\"","flags":{"seen":true,"flagged":true},"has_attachments":true}]""")
        assertEquals(1, rows.size)
        assertEquals("a@imyemail.test", rows.single().from)
        assertEquals("Hi } { \"quoted\"", rows.single().subject)
        assertFalse(rows.single().unread)
        assertTrue(rows.single().starred)
        assertTrue(rows.single().hasAttachment)
        val original = "第一行\n第二行\t\\n\u0001"
        assertEquals(original, parseBody("{\"text\":${jsonStr(original)},\"html_sanitized\":\"<p>x</p>\"}").text)
    }

    @Test
    fun markReadPreservesStarAndOtherFlags() = runBlocking<Unit> {
        var sent = ""
        val engine = JsonMailEngine { method, args ->
            when (method) {
                "list_messages" -> """[{"id":"m","flags":{"seen":false,"flagged":true,"answered":true,"draft":false,"deleted":false}}]"""
                "set_flags" -> { sent = args; "{}" }
                else -> error(method)
            }
        }
        engine.messages("inbox")
        engine.setRead("m", true)
        assertTrue(sent.contains("flagged\\\":true"))
        assertTrue(sent.contains("answered\\\":true"))
    }

    @Test
    fun archiveMovesToSameAccountAndNeverDeletes() = runBlocking<Unit> {
        val calls = mutableListOf<Pair<String, String>>()
        val engine = JsonMailEngine { method, args ->
            calls += method to args
            when (method) {
                "list_messages" -> """[{"id":"m","account_id":"a"}]"""
                "list_folders" -> """[{"id":"other","account_id":"b","role":"archive"},{"id":"archive-a","account_id":"a","role":"archive"}]"""
                "move_message" -> "{}"
                else -> error(method)
            }
        }
        engine.messages("inbox")
        engine.archive("m")
        assertEquals("move_message", calls.last().first)
        assertTrue(calls.last().second.contains("archive-a"))
        assertFalse(calls.any { it.first == "delete_message" })
    }

    @Test
    fun missingArchiveAndMoveFailureNeverFallBackToDelete() = runBlocking<Unit> {
        for (hasArchive in listOf(false, true)) {
            val calls = mutableListOf<String>()
            val engine = JsonMailEngine { method, _ ->
                calls += method
                when (method) {
                    "list_messages" -> """[{"id":"m","account_id":"a"}]"""
                    "list_folders" -> if (hasArchive) """[{"id":"archive","account_id":"a","role":"archive"}]""" else "[]"
                    "move_message" -> """{"error":"offline"}"""
                    else -> error(method)
                }
            }
            engine.messages("inbox")
            assertFailsWith<IllegalStateException> { engine.archive("m") }
            assertFalse("delete_message" in calls)
        }
    }
    @Test
    fun sendContractPreservesBothBodiesAndOmitsHtmlInPlainMode() = runBlocking {
        var request = buildJsonObject { }
        val engine = JsonMailEngine { method, args ->
            assertEquals("send", method)
            request = Json.parseToJsonElement(Json.parseToJsonElement(args).jsonObject.getValue("json").jsonPrimitive.content).jsonObject
            "{}"
        }
        val source = "**中文** \"quote\" 👋"
        engine.send("a", "qa@imyemail.test", "subject", source, "<p><strong>中文</strong></p>")
        assertEquals(source, request.getValue("body_text").jsonPrimitive.content)
        assertEquals("<p><strong>中文</strong></p>", request.getValue("body_html").jsonPrimitive.content)
        engine.send("a", "qa@imyemail.test", "subject", source)
        assertFalse("body_html" in request)
    }

}
