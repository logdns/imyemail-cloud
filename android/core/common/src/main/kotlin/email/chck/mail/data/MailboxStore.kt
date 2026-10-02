package email.imy.cloud.data

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class MailboxUiState(
    val accounts: List<Account> = emptyList(),
    val folders: List<Folder> = emptyList(),
    val messages: List<MessageRow> = emptyList(),
    val providers: List<Provider> = emptyList(),
    val selectedFolder: Folder? = null,
    val selectedMessage: MessageRow? = null,
    val body: MessageBody? = null,
    val filter: MessageFilter = MessageFilter.All,
    val status: String = "",
    val isSyncing: Boolean = false,
    val isLoading: Boolean = true,
    val isAddingAccount: Boolean = false,
    val isSending: Boolean = false,
    val isLoadingBody: Boolean = false,
    val isPreview: Boolean = false,
    val composeAccountId: String? = null,
    val offlineQueued: Int = 0,
    val showAddAccount: Boolean = false,
    val showCompose: Boolean = false,
    val draftEmail: String = "",
    val draftPassword: String = "",
    val draftHost: String = "",
    val composeTo: String = "",
    val composeSubject: String = "",
    val composeBody: String = "",
    val composeMarkdown: Boolean = true,
    val isImporting: Boolean = false,
    val searchQuery: String = "",
    val searchHits: List<MessageRow> = emptyList(),
    val lockBrandColor: Boolean = true,
    val allowRemoteImages: Boolean = false,
) {
    val visibleMessages: List<MessageRow>
        get() = when (filter) {
            MessageFilter.All -> messages
            MessageFilter.Unread -> messages.filter { it.unread }
            MessageFilter.Starred -> messages.filter { it.starred }
            MessageFilter.Attachments -> messages.filter { it.hasAttachment }
        }

    val title: String
        get() = selectedFolder?.path ?: L10n.t("统一收件箱")
}

class MailboxStore(
    private val engine: MailEngine,
    initialAllowRemoteImages: Boolean = false,
    private val persistAllowRemoteImages: (Boolean) -> Unit = {},
) {
    private val _state = MutableStateFlow(MailboxUiState(isPreview = engine is PreviewEngine, allowRemoteImages = initialAllowRemoteImages))
    val state: StateFlow<MailboxUiState> = _state.asStateFlow()
    private var folderRequest = 0L
    private var bodyRequest = 0L
    private var searchRequest = 0L

    private fun failure(t: Exception) {
        if (t is CancellationException) throw t
        _state.update { it.copy(status = t.message ?: L10n.t("操作失败，请重试")) }
    }

    suspend fun bootstrap() {
        _state.update { it.copy(isLoading = true, status = "") }
        try {
            val providers = engine.providers()
            val accounts = engine.accounts()
            _state.update { it.copy(providers = providers, accounts = accounts) }
            if (accounts.isEmpty()) {
                _state.update { it.copy(showAddAccount = true, folders = emptyList(), messages = emptyList()) }
                return
            }
            val folders = accounts.flatMap { engine.folders(it.id) }
            val folder = folders.firstOrNull { it.role == "inbox" } ?: folders.firstOrNull()
            _state.update { it.copy(folders = folders) }
            if (folder != null) openFolder(folder)
        } catch (t: Exception) {
            failure(t)
        } finally {
            _state.update { it.copy(isLoading = false) }
        }
    }

    suspend fun openFolder(folder: Folder) {
        val request = ++folderRequest
        ++bodyRequest
        _state.update { it.copy(selectedFolder = folder, selectedMessage = null, body = null,
            messages = emptyList(), isSyncing = true, isLoadingBody = false, status = "") }
        try {
            // Display the local cache before network access, including on an offline restart.
            suspend fun cached() = if (folder.role == "unified") engine.unifiedInbox() else engine.messages(folder.id)
            val cached = cached()
            if (request != folderRequest) return
            _state.update { it.copy(messages = cached) }
            engine.sync(folder.id)
            val messages = cached()
            if (request == folderRequest) _state.update { it.copy(messages = messages) }
        } catch (t: Exception) {
            if (t is CancellationException) throw t
            if (request == folderRequest) failure(t)
        } finally {
            if (request == folderRequest) _state.update { it.copy(isSyncing = false) }
        }
    }

    suspend fun open(row: MessageRow) {
        val request = ++bodyRequest
        _state.update { it.copy(selectedMessage = row, body = null, isLoadingBody = true, status = "") }
        try {
            val body = engine.body(row.id)
            if (request != bodyRequest) return
            _state.update { it.copy(body = body) }
            if (row.unread) engine.setRead(row.id, true)
            if (request == bodyRequest) _state.update { state ->
                state.copy(messages = state.messages.map { if (it.id == row.id) it.copy(unread = false) else it },
                    selectedMessage = row.copy(unread = false))
            }
        } catch (t: Exception) {
            if (t is CancellationException) throw t
            if (request == bodyRequest) failure(t)
        } finally {
            if (request == bodyRequest) _state.update { it.copy(isLoadingBody = false) }
        }
    }

    suspend fun addAccount(email: String, host: String?, password: String?, insecure: Boolean): Boolean {
        if (_state.value.isAddingAccount) return false
        _state.update { it.copy(isAddingAccount = true, status = "") }
        try {
            engine.addAccount(email, host, password, insecure)
            _state.update { it.copy(showAddAccount = false, draftEmail = "", draftPassword = "", draftHost = "") }
            bootstrap()
            return true
        } catch (t: Exception) {
            failure(t)
            return false
        } finally {
            _state.update { it.copy(isAddingAccount = false) }
        }
    }

    suspend fun send(to: String, subject: String, body: String): Boolean {
        if (_state.value.isSending || _state.value.isImporting) return false
        val state = _state.value
        val accountId = state.composeAccountId ?: state.selectedFolder?.accountId ?: state.accounts.firstOrNull()?.id
        if (accountId == null || state.accounts.none { it.id == accountId }) {
            _state.update { it.copy(status = L10n.t("请选择发件账号")) }
            return false
        }
        _state.update { it.copy(isSending = true, status = "") }
        try {
            val html = if (state.composeMarkdown) withContext(Dispatchers.Default) { MarkdownBody.render(body) } else null
            engine.send(accountId, to, subject, body, html)
            _state.update { it.copy(showCompose = false, status = "queued", composeTo = "", composeSubject = "", composeBody = "") }
            return true
        } catch (t: Exception) {
            failure(t)
            return false
        } finally {
            _state.update { it.copy(isSending = false) }
        }
    }

    suspend fun search(query: String) {
        val request = ++searchRequest
        _state.update { it.copy(searchQuery = query, searchHits = emptyList()) }
        if (query.isBlank()) return
        try {
            val hits = engine.search(query)
            if (request == searchRequest) _state.update { it.copy(searchHits = hits) }
        } catch (t: Exception) {
            if (t is CancellationException) throw t
            if (request == searchRequest) failure(t)
        }
    }

    suspend fun markRead(row: MessageRow) {
        try {
            engine.setRead(row.id, true)
            _state.update { state -> state.copy(messages = state.messages.map { if (it.id == row.id) it.copy(unread = false) else it }) }
        } catch (t: Exception) { failure(t) }
    }

    private suspend fun remove(row: MessageRow, archive: Boolean): Boolean {
        try {
            if (archive) engine.archive(row.id) else engine.delete(row.id)
            if (_state.value.selectedMessage?.id == row.id) closeThread()
            _state.update { state -> state.copy(messages = state.messages.filterNot { it.id == row.id },
                searchHits = state.searchHits.filterNot { it.id == row.id }, status = if (archive) "archived" else "deleted") }
            return true
        } catch (t: Exception) {
            failure(t)
            return false
        }
    }
    suspend fun archive(row: MessageRow) = remove(row, true)
    suspend fun delete(row: MessageRow) = remove(row, false)
    suspend fun toggleStar(row: MessageRow) {
        try {
            engine.setStarred(row.id, !row.starred)
            _state.update { state -> state.copy(messages = state.messages.map { if (it.id == row.id) it.copy(starred = !row.starred) else it }) }
        } catch (t: Exception) { failure(t) }
    }
    suspend fun refresh() {
        val folder = _state.value.selectedFolder
        if (folder != null) openFolder(folder) else bootstrap()
    }

    fun setFilter(filter: MessageFilter) = _state.update { it.copy(filter = filter) }
    fun setShowAddAccount(value: Boolean) = _state.update { it.copy(showAddAccount = value, status = "") }
    fun setShowCompose(value: Boolean) = _state.update { it.copy(showCompose = value, status = if (value) "" else it.status,
        composeAccountId = if (value) it.selectedFolder?.accountId ?: it.accounts.firstOrNull()?.id else it.composeAccountId) }
    fun setDraftEmail(value: String) = _state.update { it.copy(draftEmail = value) }
    fun setDraftPassword(value: String) = _state.update { it.copy(draftPassword = value) }
    fun setDraftHost(value: String) = _state.update { it.copy(draftHost = value) }
    fun setComposeTo(value: String) = _state.update { it.copy(composeTo = value) }
    fun setComposeSubject(value: String) = _state.update { it.copy(composeSubject = value) }
    fun setComposeMarkdown(value: Boolean) = _state.update { if (it.isSending || it.isImporting) it else it.copy(composeMarkdown = value) }
    fun setComposeBody(value: String) = _state.update { it.copy(composeBody = value) }
    fun setLockBrandColor(value: Boolean) = _state.update { it.copy(lockBrandColor = value) }
    fun setAllowRemoteImages(value: Boolean) {
        _state.update { it.copy(allowRemoteImages = value) }
        persistAllowRemoteImages(value)
    }
    fun prefillCompose(to: String = "", subject: String = "", body: String = "") {
        _state.update { it.copy(showCompose = true, composeTo = to, composeSubject = subject, composeBody = body, status = "",
            composeAccountId = it.selectedMessage?.accountId?.takeIf(String::isNotBlank)
                ?: it.selectedFolder?.accountId ?: it.accounts.firstOrNull()?.id) }
    }
    suspend fun importDocument(convert: suspend () -> String) {
        if (_state.value.isSending || _state.value.isImporting) return
        _state.update { it.copy(isImporting = true, status = "") }
        try {
            val markdown = convert().trim()
            require(markdown.isNotBlank()) { L10n.t("文档没有可提取文字；扫描 PDF 暂不支持 OCR") }
            val previous = _state.value.composeBody
            val merged = if (previous.isBlank()) markdown else previous.trimEnd() + "\n\n" + markdown
            require(merged.length <= MarkdownBody.MAX_LENGTH) { L10n.t("导入后正文超过 20 万字符，原正文已保留") }
            _state.update { it.copy(composeBody = merged, composeMarkdown = true) }
        } catch (t: Exception) { failure(t) }
        finally { _state.update { it.copy(isImporting = false) } }
    }
    fun clearStatus() = _state.update { it.copy(status = "") }
    fun closeThread() {
        ++bodyRequest
        _state.update { it.copy(selectedMessage = null, body = null, isLoadingBody = false) }
    }
}
