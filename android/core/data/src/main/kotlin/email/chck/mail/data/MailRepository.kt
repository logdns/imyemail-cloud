package email.imy.cloud.data

import kotlinx.coroutines.flow.StateFlow

class MailRepository(engine: MailEngine) {
    val store = MailboxStore(engine)
    val state: StateFlow<MailboxUiState> = store.state
}
