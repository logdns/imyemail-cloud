package email.imy.cloud.data

interface MailEngine {
    suspend fun providers(): List<Provider>
    suspend fun addAccount(
        email: String,
        host: String?,
        password: String?,
        insecure: Boolean,
        port: Int? = null,
    ): Account
    suspend fun accounts(): List<Account>
    suspend fun folders(accountId: String): List<Folder>
    suspend fun sync(folderId: String)
    suspend fun messages(folderId: String): List<MessageRow>
    suspend fun unifiedInbox(): List<MessageRow>
    suspend fun body(messageId: String): MessageBody
    suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String? = null)
    suspend fun search(query: String): List<MessageRow>
    suspend fun setRead(messageId: String, read: Boolean)
    suspend fun setStarred(messageId: String, starred: Boolean)
    suspend fun archive(messageId: String)
    suspend fun delete(messageId: String)
    suspend fun flushDueSends(): Int = 0
    suspend fun tick(accountId: String? = null, idle: Boolean = false) {}
}
