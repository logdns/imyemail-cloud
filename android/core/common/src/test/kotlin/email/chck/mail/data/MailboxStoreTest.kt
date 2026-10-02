package email.imy.cloud.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlin.test.assertNull

@OptIn(ExperimentalCoroutinesApi::class)
class MailboxStoreTest {
    @Test
    fun remoteImagePreferenceDefaultsOffRestoresAndPersistsImmediately() {
        var saved: Boolean? = null
        val initial = MailboxStore(PreviewEngine(), persistAllowRemoteImages = { saved = it })
        assertFalse(initial.state.value.allowRemoteImages)
        initial.setAllowRemoteImages(true)
        assertTrue(initial.state.value.allowRemoteImages)
        assertEquals(true, saved)

        val restored = MailboxStore(PreviewEngine(), initialAllowRemoteImages = true)
        assertTrue(restored.state.value.allowRemoteImages)
    }

    @Test
    fun bootstrapLoadsInbox() = runBlocking {
        val store = MailboxStore(PreviewEngine())
        store.bootstrap()
        val state = store.state.value
        assertEquals(1, state.accounts.size)
        assertEquals("inbox", state.folders.first { it.role == "inbox" }.role)
        assertEquals("Hello from testkit", state.messages.first().subject)
        assertTrue(state.providers.any { it.id == "qq" })
    }

    @Test
    fun sendAndAddAccountOnPreview() = runBlocking {
        val store = MailboxStore(PreviewEngine())
        store.bootstrap()
        store.send("bob@imyemail.test", "Hi", "hello")
        assertEquals("queued", store.state.value.status)
        assertFalse(store.state.value.showCompose)
        store.addAccount("dev@qq.com", null, "x", false)
        assertFalse(store.state.value.showAddAccount)
        assertTrue(store.state.value.accounts.any { it.email == "dev@qq.com" })
    }

    @Test
    fun filterUnreadAndArchive() = runBlocking {
        val store = MailboxStore(PreviewEngine())
        store.bootstrap()
        store.setFilter(MessageFilter.Unread)
        assertTrue(store.state.value.visibleMessages.all { it.unread })
        val first = store.state.value.messages.first()
        store.archive(first)
        assertFalse(store.state.value.messages.any { it.id == first.id })
    }

    @Test
    fun latestBodyWinsWhenRequestsFinishOutOfOrder() = runTest {
        val a = CompletableDeferred<MessageBody>()
        val b = CompletableDeferred<MessageBody>()
        val preview = PreviewEngine()
        val engine = object : MailEngine by preview {
            override suspend fun body(messageId: String) = if (messageId.endsWith(":1")) a.await() else b.await()
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        val rows = store.state.value.messages
        val first = launch { store.open(rows[0]) }
        runCurrent()
        val second = launch { store.open(rows[1]) }
        runCurrent()
        assertNull(store.state.value.body)
        b.complete(MessageBody("second", ""))
        second.join()
        a.complete(MessageBody("first", ""))
        first.join()
        assertEquals(rows[1].id, store.state.value.selectedMessage?.id)
        assertEquals("second", store.state.value.body?.text)
    }

    @Test
    fun closingThreadDiscardsPendingBody() = runTest {
        val response = CompletableDeferred<MessageBody>()
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun body(messageId: String) = response.await()
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        val read = launch { store.open(store.state.value.messages.first()) }
        runCurrent()
        store.closeThread()
        response.complete(MessageBody("old", ""))
        read.join()
        assertNull(store.state.value.selectedMessage)
        assertNull(store.state.value.body)
    }

    @Test
    fun offlineBootstrapKeepsCachedMessages() = runBlocking {
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun sync(folderId: String) { error("offline") }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        assertTrue(store.state.value.messages.isNotEmpty())
        assertEquals("offline", store.state.value.status)
        assertFalse(store.state.value.isSyncing)
    }

    @Test
    fun archiveFailureKeepsSelectedMailAndBody() = runBlocking {
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun archive(messageId: String) { error("UIDPLUS required") }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        val row = store.state.value.messages.first()
        store.open(row)
        val body = store.state.value.body
        assertFalse(store.archive(row))
        assertEquals(row.id, store.state.value.selectedMessage?.id)
        assertEquals(body, store.state.value.body)
        assertTrue(store.state.value.messages.any { it.id == row.id })
    }

    @Test
    fun sendFailurePreservesInputAndPreventsDuplicateSubmission() = runTest {
        val response = CompletableDeferred<Unit>()
        var attempts = 0
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String?) {
                attempts++
                response.await()
                error("offline")
            }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        store.prefillCompose("alice@imyemail.test", "subject", "keep this")
        val first = launch { store.send("alice@imyemail.test", "subject", "keep this") }
        runCurrent()
        assertTrue(store.state.value.isSending)
        assertFalse(store.send("alice@imyemail.test", "subject", "keep this"))
        response.complete(Unit)
        first.join()
        assertEquals(1, attempts)
        assertEquals("keep this", store.state.value.composeBody)
        assertTrue(store.state.value.showCompose)
        assertFalse(store.state.value.isSending)
    }

    @Test
    fun failedAccountAdditionRetainsForm() = runBlocking {
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun addAccount(email: String, host: String?, password: String?, insecure: Boolean, port: Int?): Account {
                error("authentication failed")
            }
        }
        val store = MailboxStore(engine)
        store.setShowAddAccount(true)
        store.setDraftEmail("dev@imyemail.test")
        store.setDraftPassword("test-secret")
        assertFalse(store.addAccount("dev@imyemail.test", null, "test-secret", false))
        assertTrue(store.state.value.showAddAccount)
        assertEquals("test-secret", store.state.value.draftPassword)
        assertFalse(store.state.value.isAddingAccount)
    }

    @Test
    fun selectedFolderControlsSenderAccount() = runBlocking {
        var sender = ""
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String?) { sender = accountId }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        store.addAccount("second@imyemail.test", null, "test", false)
        store.openFolder(store.state.value.folders.first { it.accountId == "second@imyemail.test" && it.role == "inbox" })
        store.setShowCompose(true)
        store.send("alice@imyemail.test", "subject", "body")
        assertEquals("second@imyemail.test", sender)
    }

    @Test
    fun switchingFoldersDiscardsLateResults() = runTest {
        val response = CompletableDeferred<Unit>()
        val preview = PreviewEngine()
        var block = false
        val engine = object : MailEngine by preview {
            override suspend fun sync(folderId: String) {
                if (block && folderId.endsWith("INBOX")) response.await()
            }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        block = true
        val first = launch { store.openFolder(store.state.value.folders.first { it.role == "inbox" }) }
        runCurrent()
        store.openFolder(store.state.value.folders.first { it.role == "sent" })
        response.complete(Unit)
        first.join()
        assertEquals("sent", store.state.value.selectedFolder?.role)
        assertTrue(store.state.value.messages.isEmpty())
    }
    @Test
    fun markdownSendUsesLatestSourceAndPlainModeOmitsHtml() = runBlocking {
        var sentBody = ""
        var sentHtml: String? = null
        val engine = object : MailEngine by PreviewEngine() {
            override suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String?) {
                sentBody = body; sentHtml = bodyHtml
            }
        }
        val store = MailboxStore(engine)
        store.bootstrap()
        store.prefillCompose("qa@imyemail.test", "subject", "old")
        store.setComposeBody("**最新 👋**")
        assertTrue(store.send(store.state.value.composeTo, store.state.value.composeSubject, store.state.value.composeBody))
        assertEquals("**最新 👋**", sentBody)
        assertTrue("<strong>最新 👋</strong>" in sentHtml.orEmpty())
        store.setComposeMarkdown(false)
        assertTrue(store.send("qa@imyemail.test", "plain", "**literal**"))
        assertNull(sentHtml)
    }

    @Test
    fun documentImportAppendsAndFailurePreservesBody() = runBlocking {
        val store = MailboxStore(PreviewEngine())
        store.setComposeBody("原正文")
        store.setComposeMarkdown(false)
        store.importDocument { "# 文档" }
        assertEquals("原正文\n\n# 文档", store.state.value.composeBody)
        assertTrue(store.state.value.composeMarkdown)
        store.importDocument { error("损坏") }
        assertEquals("原正文\n\n# 文档", store.state.value.composeBody)
        assertEquals("损坏", store.state.value.status)
        assertFalse(store.state.value.isImporting)
        store.importDocument { " " }
        assertEquals("原正文\n\n# 文档", store.state.value.composeBody)
    }

    @Test
    fun importingDisablesSendAndDuplicateImport() = runTest {
        val response = CompletableDeferred<String>()
        val store = MailboxStore(PreviewEngine())
        store.bootstrap()
        val importing = launch { store.importDocument { response.await() } }
        runCurrent()
        assertTrue(store.state.value.isImporting)
        assertFalse(store.send("qa@imyemail.test", "s", "b"))
        store.importDocument { error("duplicate must not run") }
        response.complete("text")
        importing.join()
        assertEquals("text", store.state.value.composeBody)
    }

}
