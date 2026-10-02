package email.imy.cloud.data

import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*

class JsonMailEngine(
    private val invoke: suspend (method: String, args: String) -> String,
) : MailEngine {
    // A single native handle is shared by the UI and WorkManager. Never call it on the UI thread.
    private val calls = Mutex()
    private val mutations = Mutex()
    private val summaries = ConcurrentHashMap<String, JsonObject>()

    private suspend fun call(method: String, args: String = "{}"): JsonElement = withContext(Dispatchers.IO) {
        calls.withLock {
            val value = Json.parseToJsonElement(invoke(method, args))
            if (value is JsonObject && "error" in value) {
                error(value.string("error").ifBlank { L10n.t("邮件服务操作失败") })
            }
            value
        }
    }

    override suspend fun providers(): List<Provider> = parseProviders(call("list_providers").toString())

    override suspend fun addAccount(email: String, host: String?, password: String?, insecure: Boolean, port: Int?): Account {
        val request = buildJsonObject {
            put("email", email)
            put("accept_invalid_certs", insecure)
            if (!host.isNullOrBlank()) put("imap_host", host)
            if (port != null) put("imap_port", port)
            if (!password.isNullOrBlank()) put("password", password)
        }
        val args = buildJsonObject { put("json", request.toString()) }.toString()
        val probe = call("test_account", args).jsonObject["steps"]?.jsonArray.orEmpty()
        check(probe.isNotEmpty() && probe.all { it.jsonObject.string("status") == "ok" }) {
            L10n.t("连接测试未通过，请检查服务器、网络和邮箱凭据")
        }
        return parseAccount(call("add_account", args).toString()) ?: error(L10n.t("账号响应无效"))
    }

    override suspend fun accounts() = parseAccounts(call("list_accounts").toString())
    override suspend fun folders(accountId: String) =
        parseFolders(call("list_folders", """{"account_id":${jsonStr(accountId)}}""").toString())
    override suspend fun sync(folderId: String) {
        call("sync_folder", """{"folder_id":${jsonStr(folderId)}}""")
    }
    private fun rows(value: JsonElement): List<MessageRow> {
        value.jsonArray.forEach { summaries[it.jsonObject.string("id")] = it.jsonObject }
        return parseMessages(value.toString())
    }
    override suspend fun messages(folderId: String) = rows(call("list_messages", """{"folder_id":${jsonStr(folderId)},"offset":0,"limit":50}"""))
    override suspend fun unifiedInbox() = rows(call("unified_inbox", """{"offset":0,"limit":50}"""))
    override suspend fun body(messageId: String) = parseBody(call("body", """{"message_id":${jsonStr(messageId)}}""").toString())
    override suspend fun send(accountId: String, to: String, subject: String, body: String, bodyHtml: String?) {
        val request = buildJsonObject {
            put("account_id", accountId)
            putJsonArray("to") { add(to) }
            put("subject", subject)
            put("body_text", body)
            if (bodyHtml != null) put("body_html", bodyHtml)
            put("undo_window_secs", 10)
        }
        call("send", buildJsonObject { put("json", request.toString()) }.toString())
    }
    override suspend fun search(query: String): List<MessageRow> {
        val ids = call("search", """{"query":${jsonStr(query)}}""").jsonArray.map { it.jsonObject.string("message_id") }.toSet()
        return unifiedInbox().filter { it.id in ids }
    }
    private suspend fun changeFlag(messageId: String, flag: String, value: Boolean) = mutations.withLock {
        val summary = summaries[messageId] ?: error(L10n.t("请刷新邮件后重试"))
        val previous = summary["flags"] as? JsonObject ?: summary
        val flags = buildJsonObject {
            for (key in listOf("seen", "flagged", "answered", "draft", "deleted")) {
                put(key, if (key == flag) value else previous.boolean(key))
            }
        }
        call("set_flags", buildJsonObject {
            put("message_id", messageId)
            put("flags", flags.toString())
        }.toString())
        summaries[messageId] = JsonObject(summary + ("flags" to flags))
    }
    override suspend fun setRead(messageId: String, read: Boolean) { changeFlag(messageId, "seen", read) }
    override suspend fun setStarred(messageId: String, starred: Boolean) { changeFlag(messageId, "flagged", starred) }
    override suspend fun archive(messageId: String) {
        val summary = summaries[messageId] ?: error(L10n.t("请刷新邮件后重试"))
        val accountId = summary.string("account_id").ifBlank { error(L10n.t("无法确认邮件所属账号")) }
        val destination = folders(accountId).firstOrNull { it.role == "archive" && it.accountId == accountId }
            ?: error(L10n.t("此账号没有归档文件夹，邮件已保留"))
        call("move_message", """{"message_id":${jsonStr(messageId)},"dest_folder_id":${jsonStr(destination.id)}}""")
    }
    override suspend fun delete(messageId: String) {
        call("delete_message", """{"message_id":${jsonStr(messageId)}}""")
    }
    override suspend fun flushDueSends(): Int = call("flush_due_sends").jsonObject.int("flushed")
    override suspend fun tick(accountId: String?, idle: Boolean) {
        call("tick", buildJsonObject {
            put("idle", idle)
            if (!accountId.isNullOrBlank()) put("account_id", accountId)
        }.toString())
    }
}

private fun JsonObject.string(key: String): String = (this[key] as? JsonPrimitive)?.contentOrNull.orEmpty()
private fun JsonObject.boolean(key: String): Boolean = (this[key] as? JsonPrimitive)?.booleanOrNull ?: false
private fun JsonObject.int(key: String): Int = (this[key] as? JsonPrimitive)?.intOrNull ?: 0
private fun objects(json: String) = Json.parseToJsonElement(json).jsonArray.map { it.jsonObject }
internal fun jsonStr(value: String): String = JsonPrimitive(value).toString()
internal fun parseAccount(json: String): Account? = accountFrom(Json.parseToJsonElement(json).jsonObject)
internal fun parseAccounts(json: String) = objects(json).mapNotNull(::accountFrom)
private fun accountFrom(obj: JsonObject): Account? {
    val id = obj.string("id").ifBlank { return null }
    return Account(id, obj.string("email"), obj.string("display_name"), obj.string("provider_id").ifBlank { "custom" })
}
internal fun parseFolders(json: String) = objects(json).map { obj ->
    Folder(obj.string("id"), obj.string("path"), obj.string("role"), obj.int("unread_count"), obj.string("account_id"))
}
internal fun parseMessages(json: String) = objects(json).map { obj ->
    val flags = obj["flags"] as? JsonObject ?: obj
    val sender = when (val from = obj["from"]) {
        is JsonArray -> from.joinToString(", ") { it.jsonObject.string("email") }
        is JsonPrimitive -> from.contentOrNull.orEmpty()
        else -> ""
    }
    val date = (obj["date_unix"] as? JsonPrimitive)?.longOrNull?.let {
        DateTimeFormatter.ofPattern("MM-dd HH:mm").withZone(ZoneId.systemDefault()).format(Instant.ofEpochSecond(it))
    } ?: obj.string("date")
    MessageRow(
        id = obj.string("id"), subject = obj.string("subject"), from = sender,
        snippet = obj.string("snippet"), unread = !flags.boolean("seen"), starred = flags.boolean("flagged"),
        hasAttachment = obj.boolean("has_attachments") || obj.boolean("has_attachment"),
        dateLabel = date, accountId = obj.string("account_id"),
    )
}
internal fun parseBody(json: String): MessageBody {
    val obj = Json.parseToJsonElement(json).jsonObject
    return MessageBody(obj.string("text"), obj.string("html_sanitized"), obj.int("remote_blocked"))
}
internal fun parseProviders(json: String) = objects(json).map { obj ->
    val id = obj.string("id")
    val preset = BuiltinProviders.all.firstOrNull { it.id == id }
    Provider(
        id = id, displayName = L10n.t(obj.string("display_name_zh").ifBlank { obj.string("display_name").ifBlank { id } }),
        imapHost = obj.string("imap_host"), authKind = obj.string("auth_kind"),
        displayNameEn = obj.string("display_name_en").ifBlank { id },
        imapPort = obj.int("imap_port").takeIf { it > 0 } ?: 993,
        smtpHost = obj.string("smtp_host"), smtpPort = obj.int("smtp_port").takeIf { it > 0 } ?: 465,
        helpUrl = obj.string("help_url").ifBlank { null },
        domains = (obj["domains"] as? JsonArray)?.map { it.jsonPrimitive.content } ?: preset?.domains.orEmpty(),
        group = preset?.group ?: ProviderGroup.International,
    )
}
