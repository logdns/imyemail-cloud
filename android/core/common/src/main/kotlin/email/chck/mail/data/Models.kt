package email.imy.cloud.data

data class Account(
    val id: String,
    val email: String,
    val displayName: String,
    val providerId: String,
    val color: Long = 0xFF3D6BFE,
    val unread: Int = 0,
)

data class Folder(
    val id: String,
    val path: String,
    val role: String,
    val unread: Int,
    val accountId: String = "",
)

data class MessageRow(
    val id: String,
    val subject: String,
    val from: String,
    val snippet: String,
    val unread: Boolean,
    val starred: Boolean = false,
    val hasAttachment: Boolean = false,
    val dateLabel: String = "",
    val accountId: String = "",
    val accountColor: Long = 0xFF3D6BFE,
    val threadCount: Int = 1,
)

data class MessageBody(
    val text: String,
    val html: String,
    val remoteContentCount: Int = 0,
)

data class Provider(
    val id: String,
    val displayName: String,
    val imapHost: String,
    val authKind: String,
    val displayNameEn: String = displayName,
    val imapPort: Int = 993,
    val smtpHost: String = "",
    val smtpPort: Int = 465,
    val helpUrl: String? = null,
    val domains: List<String> = emptyList(),
    val group: ProviderGroup = ProviderGroup.International,
)

enum class ProviderGroup { International, Domestic }

enum class MessageFilter { All, Unread, Starred, Attachments }

enum class AccountState { Online, Syncing, Error, AuthRequired, Offline }
