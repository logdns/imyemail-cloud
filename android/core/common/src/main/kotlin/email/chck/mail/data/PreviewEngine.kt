package email.imy.cloud.data

class PreviewEngine : MailEngine {
    private val palette = longArrayOf(0xFF3D6BFE, 0xFF0F9D58, 0xFFDB4437, 0xFFF4B400, 0xFF9C27B0)
    private val added = mutableListOf(
        Account("preview", "dev@imyemail.test", "Dev", "custom", palette[0], unread = 2),
    )
    private val rows = mutableListOf(
        MessageRow("preview:INBOX:1", "Hello from testkit", "alice@imyemail.test", "Hello", true, dateLabel = "14:02", accountId = "preview", accountColor = palette[0]),
        MessageRow("preview:INBOX:2", "同步报告", "ops@imy.email", "昨夜文件夹增量完成", true, starred = true, dateLabel = "昨天", accountId = "preview", accountColor = palette[0]),
        MessageRow("preview:INBOX:3", "附件合同", "legal@imyemail.test", "请查收 PDF", false, hasAttachment = true, dateLabel = "周一", accountId = "preview", accountColor = palette[0]),
        MessageRow("preview:INBOX:4", "已读摘要", "bob@imyemail.test", "上周已处理", false, dateLabel = "上周", accountId = "preview", accountColor = palette[0]),
    )

    override suspend fun providers() = BuiltinProviders.all

    override suspend fun addAccount(
        email: String,
        host: String?,
        password: String?,
        insecure: Boolean,
        port: Int?,
    ): Account {
        val provider = BuiltinProviders.match(email)
        val color = palette[added.size % palette.size]
        val acc = Account(
            id = email,
            email = email,
            displayName = email.substringBefore("@"),
            providerId = provider?.id ?: "custom",
            color = color,
        )
        added.add(acc)
        return acc
    }

    override suspend fun accounts() = added.toList()

    override suspend fun folders(accountId: String) = listOf(
        Folder("$accountId:unified", "统一收件箱", "unified", unreadOf(accountId), accountId),
        Folder("$accountId:INBOX", "收件箱", "inbox", unreadOf(accountId), accountId),
        Folder("$accountId:Starred", "星标", "starred", 0, accountId),
        Folder("$accountId:Snoozed", "稍后", "snooze", 0, accountId),
        Folder("$accountId:Drafts", "草稿", "drafts", 0, accountId),
        Folder("$accountId:Sent", "已发送", "sent", 0, accountId),
        Folder("$accountId:Junk", "垃圾邮件", "junk", 0, accountId),
        Folder("$accountId:Trash", "废纸篓", "trash", 0, accountId),
    )

    override suspend fun sync(folderId: String) = Unit

    override suspend fun messages(folderId: String): List<MessageRow> {
        val role = folderId.substringAfterLast(":")
        return when (role) {
            "Starred" -> rows.filter { it.starred }
            "unified", "INBOX" -> rows.toList()
            else -> emptyList()
        }
    }

    override suspend fun unifiedInbox() = rows.toList()

    override suspend fun body(messageId: String): MessageBody {
        val row = rows.firstOrNull { it.id == messageId }
        val subject = row?.subject ?: "Hello from testkit"
        return MessageBody(
            text = "$subject\n\nHello from testkit",
            html = "<p>$subject</p><p>Hello from testkit</p><img src=\"https://track.example/p.gif\">",
            remoteContentCount = 1,
        )
    }

    override suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String?) = Unit

    override suspend fun search(query: String): List<MessageRow> {
        val q = query.lowercase()
        if (q.isBlank()) return emptyList()
        return rows.filter {
            it.subject.lowercase().contains(q) ||
                it.from.lowercase().contains(q) ||
                it.snippet.lowercase().contains(q)
        }
    }

    override suspend fun setRead(messageId: String, read: Boolean) {
        update(messageId) { it.copy(unread = !read) }
    }

    override suspend fun setStarred(messageId: String, starred: Boolean) {
        update(messageId) { it.copy(starred = starred) }
    }

    override suspend fun archive(messageId: String) {
        rows.removeAll { it.id == messageId }
    }

    override suspend fun delete(messageId: String) {
        rows.removeAll { it.id == messageId }
    }

    private fun unreadOf(accountId: String) = rows.count { it.unread && (it.accountId == accountId || accountId == "preview") }

    private fun update(id: String, transform: (MessageRow) -> MessageRow) {
        val i = rows.indexOfFirst { it.id == id }
        if (i >= 0) rows[i] = transform(rows[i])
    }
}
