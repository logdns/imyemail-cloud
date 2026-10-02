package email.imy.cloud

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import email.imy.cloud.data.Folder
import email.imy.cloud.data.MailboxStore
import email.imy.cloud.data.MessageFilter
import email.imy.cloud.data.MessageRow
import javax.inject.Inject
import kotlinx.coroutines.launch

@HiltViewModel
class MailboxViewModel @Inject constructor(
    private val store: MailboxStore,
) : ViewModel() {
    val state = store.state

    init {
        viewModelScope.launch { store.bootstrap() }
    }

    fun openFolder(folder: Folder) = viewModelScope.launch { store.openFolder(folder) }
    fun open(row: MessageRow) = viewModelScope.launch { store.open(row) }
    fun addAccount(onSuccess: () -> Unit = {}) = viewModelScope.launch {
        val s = store.state.value
        if (store.addAccount(s.draftEmail, s.draftHost.ifBlank { null }, s.draftPassword.ifBlank { null }, false)) onSuccess()
    }
    fun send(onSuccess: () -> Unit = {}) = viewModelScope.launch {
        val s = store.state.value
        if (store.send(s.composeTo, s.composeSubject, s.composeBody)) onSuccess()
    }
    fun search(query: String) = viewModelScope.launch { store.search(query) }
    fun markRead(row: MessageRow) = viewModelScope.launch { store.markRead(row) }
    fun archive(row: MessageRow, onSuccess: () -> Unit = {}) = viewModelScope.launch { if (store.archive(row)) onSuccess() }
    fun delete(row: MessageRow, onSuccess: () -> Unit = {}) = viewModelScope.launch { if (store.delete(row)) onSuccess() }
    fun toggleStar(row: MessageRow) = viewModelScope.launch { store.toggleStar(row) }
    fun refresh() = viewModelScope.launch { store.refresh() }
    fun setFilter(filter: MessageFilter) = store.setFilter(filter)
    fun setShowAddAccount(value: Boolean) = store.setShowAddAccount(value)
    fun setShowCompose(value: Boolean) = store.setShowCompose(value)
    fun setDraftEmail(value: String) = store.setDraftEmail(value)
    fun setDraftPassword(value: String) = store.setDraftPassword(value)
    fun setDraftHost(value: String) = store.setDraftHost(value)
    fun setComposeTo(value: String) = store.setComposeTo(value)
    fun setComposeSubject(value: String) = store.setComposeSubject(value)
    fun importDocument(context: android.content.Context, uri: android.net.Uri) = viewModelScope.launch {
        store.importDocument { DocumentImporter.convert(context.applicationContext, uri) }
    }
    fun setComposeMarkdown(value: Boolean) = store.setComposeMarkdown(value)
    fun setComposeBody(value: String) = store.setComposeBody(value)
    fun setLockBrandColor(value: Boolean) = store.setLockBrandColor(value)
    fun setAllowRemoteImages(value: Boolean) = store.setAllowRemoteImages(value)
    fun prefillCompose(to: String = "", subject: String = "", body: String = "") = store.prefillCompose(to, subject, body)
    fun clearStatus() = store.clearStatus()
    fun closeThread() = store.closeThread()
}
