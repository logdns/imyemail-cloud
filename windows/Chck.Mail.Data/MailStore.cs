using Chck.Mail.Core;
using Microsoft.Data.Sqlite;
using System.Text.Json;

namespace Chck.Mail.Data;

public sealed class MailStore : IDisposable
{
    public const int SchemaVersion = 4;
    private readonly SqliteConnection _conn;

    public MailStore(string path) : this(path, false) { }

    private MailStore(string path, bool readOnly)
    {
        var builder = new SqliteConnectionStringBuilder
        {
            DataSource = path,
            ForeignKeys = true,
            Mode = readOnly ? SqliteOpenMode.ReadOnly : SqliteOpenMode.ReadWriteCreate,
            Pooling = !readOnly,
            DefaultTimeout = readOnly ? 1 : 30,
        };
        _conn = new SqliteConnection(builder.ToString());
        _conn.Open();
        if (readOnly) return;
        using (var pragma = _conn.CreateCommand())
        {
            pragma.CommandText = "PRAGMA journal_mode=WAL;";
            pragma.ExecuteNonQuery();
        }

        Migrate();
    }

    public static MailStore OpenReadOnly(string path) => new(path, true);

    public static MailStore OpenInMemory()
    {
        var store = new MailStore(":memory:");
        return store;
    }

    public void Dispose() => _conn.Dispose();

    public int UserVersion()
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "PRAGMA user_version;";
        return Convert.ToInt32(cmd.ExecuteScalar());
    }

    public uint? GetFolderUidValidity(string folderId)
    {
        using var command = _conn.CreateCommand();
        command.CommandText = "SELECT uidvalidity FROM folders WHERE id = $id";
        command.Parameters.AddWithValue("$id", folderId);
        var value = command.ExecuteScalar();
        return value is null or DBNull ? null : checked((uint)Convert.ToInt64(value));
    }

    public void SetFolderUidValidity(string folderId, uint validity)
    {
        if (validity == 0) throw EngineException.Invalid("missing UIDVALIDITY");
        using var transaction = _conn.BeginTransaction();
        using var previous = _conn.CreateCommand();
        previous.Transaction = transaction;
        previous.CommandText = "SELECT uidvalidity FROM folders WHERE id = $id";
        previous.Parameters.AddWithValue("$id", folderId);
        var old = previous.ExecuteScalar();
        // Legacy unknown identities are also invalidated before trusting new UIDs.
        if (old is null or DBNull || Convert.ToInt64(old) != validity)
        {
            using var invalidate = _conn.CreateCommand();
            invalidate.Transaction = transaction;
            invalidate.CommandText = """
                DELETE FROM search_index WHERE message_id IN (SELECT id FROM messages WHERE folder_id = $id);
                DELETE FROM messages WHERE folder_id = $id;
                UPDATE folders SET sync_cursor = NULL WHERE id = $id;
                """;
            invalidate.Parameters.AddWithValue("$id", folderId);
            invalidate.ExecuteNonQuery();
        }
        using var update = _conn.CreateCommand();
        update.Transaction = transaction;
        update.CommandText = "UPDATE folders SET uidvalidity = $validity WHERE id = $id";
        update.Parameters.AddWithValue("$validity", validity);
        update.Parameters.AddWithValue("$id", folderId);
        update.ExecuteNonQuery();
        transaction.Commit();
    }

    public void InsertAccount(Account account) => InsertAccount(account, null);

    public void InsertAccountConfiguration(Account account, string configuration, Action saveCredentials)
    {
        using var transaction = _conn.BeginTransaction();
        InsertAccount(account, transaction);
        using var settings = _conn.CreateCommand();
        settings.Transaction = transaction;
        settings.CommandText = "INSERT INTO settings(key, value) VALUES ($key, $value)";
        settings.Parameters.AddWithValue("$key", $"imap:{account.Id}");
        settings.Parameters.AddWithValue("$value", configuration);
        settings.ExecuteNonQuery();
        saveCredentials();
        transaction.Commit();
    }

    private void InsertAccount(Account account, SqliteTransaction? transaction)
    {
        using var cmd = _conn.CreateCommand();
        cmd.Transaction = transaction;
        cmd.CommandText = """
            INSERT INTO accounts (id, email, display_name, provider, mode, color, sync_scope, state, created_at)
            VALUES ($id, $email, $name, $provider, $mode, $color, 'last30Days', $state, strftime('%s','now'))
            """;
        cmd.Parameters.AddWithValue("$id", account.Id);
        cmd.Parameters.AddWithValue("$email", account.Email);
        cmd.Parameters.AddWithValue("$name", account.DisplayName);
        cmd.Parameters.AddWithValue("$provider", account.ProviderId);
        cmd.Parameters.AddWithValue("$mode", account.Mode);
        cmd.Parameters.AddWithValue("$color", account.Color);
        cmd.Parameters.AddWithValue("$state", account.State);
        cmd.ExecuteNonQuery();
    }

    public IReadOnlyList<Account> ListAccounts()
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, email, display_name, provider, mode, color, state FROM accounts ORDER BY created_at";
        using var reader = cmd.ExecuteReader();
        var list = new List<Account>();
        while (reader.Read())
        {
            list.Add(new Account(
                reader.GetString(0),
                reader.GetString(1),
                reader.GetString(2),
                reader.GetString(3),
                reader.GetString(4),
                reader.GetString(5),
                reader.GetString(6)));
        }

        return list;
    }

    public Account? GetAccount(string id)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, email, display_name, provider, mode, color, state FROM accounts WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", id);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return new Account(
            reader.GetString(0),
            reader.GetString(1),
            reader.GetString(2),
            reader.GetString(3),
            reader.GetString(4),
            reader.GetString(5),
            reader.GetString(6));
    }

    public void UpdateAccountConfiguration(Account account, string configuration, Action saveCredentials)
    {
        using var transaction = _conn.BeginTransaction();
        using var cmd = _conn.CreateCommand();
        cmd.Transaction = transaction;
        cmd.CommandText = "UPDATE accounts SET display_name = $name WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", account.Id);
        cmd.Parameters.AddWithValue("$name", account.DisplayName);
        if (cmd.ExecuteNonQuery() != 1) throw EngineException.NotFound();
        using var settings = _conn.CreateCommand();
        settings.Transaction = transaction;
        settings.CommandText = "INSERT INTO settings(key, value) VALUES ($key, $value) ON CONFLICT(key) DO UPDATE SET value=excluded.value";
        settings.Parameters.AddWithValue("$key", $"imap:{account.Id}");
        settings.Parameters.AddWithValue("$value", configuration);
        settings.ExecuteNonQuery();
        saveCredentials();
        transaction.Commit();
    }

    public void DeleteAccount(string id, Action? deleteCredentials = null)
    {
        using var transaction = _conn.BeginTransaction();
        using var cmd = _conn.CreateCommand();
        cmd.Transaction = transaction;
        // FTS and settings have no foreign keys; remove them before the cascades.
        cmd.CommandText = """
            DELETE FROM search_index WHERE message_id IN (SELECT id FROM messages WHERE account_id = $id);
            DELETE FROM settings WHERE key = $key;
            DELETE FROM accounts WHERE id = $id;
            """;
        cmd.Parameters.AddWithValue("$id", id);
        cmd.Parameters.AddWithValue("$key", $"imap:{id}");
        cmd.ExecuteNonQuery();
        deleteCredentials?.Invoke();
        transaction.Commit();
    }

    public void SetKv(string key, string value)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO settings (key, value) VALUES ($k, $v)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value
            """;
        cmd.Parameters.AddWithValue("$k", key);
        cmd.Parameters.AddWithValue("$v", value);
        cmd.ExecuteNonQuery();
    }

    public string? GetKv(string key)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT value FROM settings WHERE key = $k";
        cmd.Parameters.AddWithValue("$k", key);
        return cmd.ExecuteScalar() as string;
    }

    public void DeleteKv(string key)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "DELETE FROM settings WHERE key = $k";
        cmd.Parameters.AddWithValue("$k", key);
        cmd.ExecuteNonQuery();
    }

    public void UpsertFolder(Folder folder, string accountId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO folders (id, account_id, remote_name, path, role, unread_count, total_count)
            VALUES ($id, $aid, $name, $path, $role, $unread, $total)
            ON CONFLICT(id) DO UPDATE SET
                remote_name=excluded.remote_name,
                path=excluded.path,
                role=excluded.role,
                unread_count=excluded.unread_count,
                total_count=excluded.total_count
            """;
        cmd.Parameters.AddWithValue("$id", folder.Id);
        cmd.Parameters.AddWithValue("$aid", accountId);
        cmd.Parameters.AddWithValue("$name", folder.Path);
        cmd.Parameters.AddWithValue("$path", folder.Path);
        cmd.Parameters.AddWithValue("$role", folder.Role);
        cmd.Parameters.AddWithValue("$unread", (long)folder.Unread);
        cmd.Parameters.AddWithValue("$total", (long)folder.Total);
        cmd.ExecuteNonQuery();
    }

    public IReadOnlyList<Folder> ListFolders(string accountId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT id, path, role, unread_count, total_count
            FROM folders WHERE account_id = $aid ORDER BY path
            """;
        cmd.Parameters.AddWithValue("$aid", accountId);
        using var reader = cmd.ExecuteReader();
        var list = new List<Folder>();
        while (reader.Read())
        {
            list.Add(new Folder(
                reader.GetString(0),
                reader.GetString(1),
                reader.GetString(2),
                (uint)reader.GetInt64(3),
                (uint)reader.GetInt64(4)));
        }

        return list;
    }

    public Folder? GetFolder(string id)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, path, role, unread_count, total_count, account_id FROM folders WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", id);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return new Folder(
            reader.GetString(0),
            reader.GetString(1),
            reader.GetString(2),
            (uint)reader.GetInt64(3),
            (uint)reader.GetInt64(4));
    }

    public string? FolderAccountId(string folderId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT account_id FROM folders WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", folderId);
        return cmd.ExecuteScalar() as string;
    }

    public uint FolderCursor(string folderId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT sync_cursor FROM folders WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", folderId);
        var raw = cmd.ExecuteScalar() as string;
        return uint.TryParse(raw, out var n) ? n : 1u;
    }

    public void SetFolderCursor(string folderId, uint uid, uint total, uint unread)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "UPDATE folders SET sync_cursor = $c, total_count = $t, unread_count = $u WHERE id = $id";
        cmd.Parameters.AddWithValue("$c", uid.ToString());
        cmd.Parameters.AddWithValue("$t", (long)total);
        cmd.Parameters.AddWithValue("$u", (long)unread);
        cmd.Parameters.AddWithValue("$id", folderId);
        cmd.ExecuteNonQuery();
    }

    public void UpsertMessage(string accountId, string folderId, uint uid, string subject, string fromJson, long dateUnix, string snippet, MailFlags flags, bool hasAttachments, string? messageId, string? threadId, string? toJson = null, string? ccJson = null, string? replyToJson = null)
    {
        // Preserve a legacy row's primary key on upsert, including its body/attachment links.
        using var existing = _conn.CreateCommand();
        existing.CommandText = "SELECT id FROM messages WHERE account_id = $aid AND folder_id = $fid AND uid = $uid";
        existing.Parameters.AddWithValue("$aid", accountId);
        existing.Parameters.AddWithValue("$fid", folderId);
        existing.Parameters.AddWithValue("$uid", (long)uid);
        var validity = GetFolderUidValidity(folderId);
        var id = existing.ExecuteScalar() as string
            ?? (validity is > 0 ? $"{folderId}:{validity}:{uid}" : $"{folderId}:{uid}");
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO messages (
                id, account_id, folder_id, uid, message_id, thread_id, subject, from_json, date, snippet, flags, has_attachments, body_state, to_json, cc_json, reply_to_json)
            VALUES ($id, $aid, $fid, $uid, $mid, $tid, $subj, $from, $date, $snip, $flags, $att, 'headers', $to, $cc, $reply)
            ON CONFLICT(account_id, folder_id, uid) DO UPDATE SET
                subject=excluded.subject,
                from_json=excluded.from_json,
                date=excluded.date,
                snippet=excluded.snippet,
                flags=excluded.flags,
                has_attachments=excluded.has_attachments,
                message_id=excluded.message_id,
                thread_id=excluded.thread_id,
                to_json=COALESCE(excluded.to_json, messages.to_json),
                cc_json=COALESCE(excluded.cc_json, messages.cc_json),
                reply_to_json=COALESCE(excluded.reply_to_json, messages.reply_to_json)
            """;
        cmd.Parameters.AddWithValue("$id", id);
        cmd.Parameters.AddWithValue("$aid", accountId);
        cmd.Parameters.AddWithValue("$fid", folderId);
        cmd.Parameters.AddWithValue("$uid", (long)uid);
        cmd.Parameters.AddWithValue("$mid", (object?)messageId ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$tid", (object?)threadId ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$subj", subject);
        cmd.Parameters.AddWithValue("$from", fromJson);
        cmd.Parameters.AddWithValue("$date", dateUnix);
        cmd.Parameters.AddWithValue("$snip", snippet);
        cmd.Parameters.AddWithValue("$flags", flags.ToBits());
        cmd.Parameters.AddWithValue("$att", hasAttachments ? 1 : 0);
        cmd.Parameters.AddWithValue("$to", (object?)toJson ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$cc", (object?)ccJson ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$reply", (object?)replyToJson ?? DBNull.Value);
        cmd.ExecuteNonQuery();
        IndexMessage(id, subject, fromJson, snippet);
    }

    public bool NeedsEnvelopeMetadata(string folderId)
    {
        using var command = _conn.CreateCommand();
        command.CommandText = "SELECT EXISTS(SELECT 1 FROM messages WHERE folder_id = $folder AND reply_to_json IS NULL)";
        command.Parameters.AddWithValue("$folder", folderId);
        return Convert.ToInt64(command.ExecuteScalar()) != 0;
    }

    public IReadOnlyList<MessageRow> ListMessages(string folderId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT id, subject, from_json, snippet, flags, date, has_attachments, thread_id, to_json, cc_json, reply_to_json
            FROM messages WHERE folder_id = $fid ORDER BY date DESC, uid DESC LIMIT 50
            """;
        cmd.Parameters.AddWithValue("$fid", folderId);
        return ReadRows(cmd);
    }

    public IReadOnlyList<MessageRow> UnifiedInbox()
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT m.id, m.subject, m.from_json, m.snippet, m.flags, m.date, m.has_attachments, m.thread_id, m.to_json, m.cc_json, m.reply_to_json
            FROM messages m JOIN folders f ON f.id = m.folder_id
            WHERE f.role = 'inbox'
            ORDER BY m.date DESC, m.uid DESC LIMIT 50
            """;
        return ReadRows(cmd);
    }

    public IReadOnlyList<MessageRow> Search(string query)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT m.id, m.subject, m.from_json, m.snippet, m.flags, m.date, m.has_attachments, m.thread_id, m.to_json, m.cc_json, m.reply_to_json
            FROM search_index s
            JOIN messages m ON m.id = s.message_id
            WHERE search_index MATCH $q
            LIMIT 50
            """;
        cmd.Parameters.AddWithValue("$q", query);
        try
        {
            return ReadRows(cmd);
        }
        catch (SqliteException)
        {
            return [];
        }
    }

    public (string AccountId, string FolderId, uint Uid, string FolderPath)? GetMessageMeta(string messageId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT m.account_id, m.folder_id, m.uid, f.path
            FROM messages m JOIN folders f ON f.id = m.folder_id WHERE m.id = $id
            """;
        cmd.Parameters.AddWithValue("$id", messageId);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return (reader.GetString(0), reader.GetString(1), (uint)reader.GetInt64(2), reader.GetString(3));
    }

    public MailFlags? GetMessageFlags(string messageId)
    {
        using var command = _conn.CreateCommand();
        command.CommandText = "SELECT flags FROM messages WHERE id = $id";
        command.Parameters.AddWithValue("$id", messageId);
        var value = command.ExecuteScalar();
        return value is null or DBNull ? null : MailFlags.FromBits(Convert.ToInt64(value));
    }

    public void SetMessageFlags(string messageId, MailFlags flags)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "UPDATE messages SET flags = $f WHERE id = $id";
        cmd.Parameters.AddWithValue("$f", flags.ToBits());
        cmd.Parameters.AddWithValue("$id", messageId);
        cmd.ExecuteNonQuery();
        using var counts = _conn.CreateCommand();
        counts.CommandText = """
            UPDATE folders SET unread_count = (SELECT COUNT(*) FROM messages WHERE folder_id = folders.id AND (flags & 1) = 0)
            WHERE id = (SELECT folder_id FROM messages WHERE id = $id)
            """;
        counts.Parameters.AddWithValue("$id", messageId);
        counts.ExecuteNonQuery();
    }

    public void DeleteMessage(string messageId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "DELETE FROM messages WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", messageId);
        cmd.ExecuteNonQuery();
    }

    public (uint Total, uint Unread) MessageCount(string folderId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT COUNT(*), SUM(CASE WHEN (flags & 1) = 0 THEN 1 ELSE 0 END) FROM messages WHERE folder_id = $id";
        cmd.Parameters.AddWithValue("$id", folderId);
        using var reader = cmd.ExecuteReader();
        reader.Read();
        var total = (uint)reader.GetInt64(0);
        var unread = reader.IsDBNull(1) ? 0u : (uint)reader.GetInt64(1);
        return (total, unread);
    }

    /// <summary>Apply a complete, stable UID/flags snapshot after the network layer verifies it.</summary>
    public void ReconcileFolderSnapshot(string folderId, uint validity, IReadOnlyDictionary<uint, MailFlags> remoteFlags)
    {
        if (validity == 0 || GetFolderUidValidity(folderId) != validity || remoteFlags.ContainsKey(0))
            throw EngineException.Invalid("mailbox snapshot identity changed");
        var folder = GetFolder(folderId) ?? throw EngineException.NotFound();
        var accountId = FolderAccountId(folderId) ?? throw EngineException.NotFound();
        var effective = new Dictionary<uint, MailFlags>(remoteFlags);
        // Normal synchronization flushes operations first. If a local operation is
        // still pending, keep its intent while refreshing the other server flags.
        foreach (var (_, pendingAccount, type, payload) in ListPendingOps())
        {
            if (pendingAccount != accountId) continue;
            try
            {
                using var document = JsonDocument.Parse(payload);
                var root = document.RootElement;
                if (!root.TryGetProperty("source", out var source) || source.GetString() != folder.Path ||
                    !root.TryGetProperty("uidvalidity", out var identity) || !identity.TryGetUInt32(out var expected) || expected != validity ||
                    !root.TryGetProperty("uid", out var uidValue) || !uidValue.TryGetUInt32(out var uid) || !effective.TryGetValue(uid, out var flags)) continue;
                if (type == "delete") effective.Remove(uid);
                else if (type == "mark_read") effective[uid] = flags with { Seen = true };
                else if (type == "flag" && root.TryGetProperty("flags", out var bits) && bits.TryGetInt64(out var value))
                    effective[uid] = MailFlags.FromBits(value);
            }
            catch (JsonException) { }
            catch (InvalidOperationException) { }
        }

        using var transaction = _conn.BeginTransaction();
        using (var identityCheck = _conn.CreateCommand())
        {
            identityCheck.Transaction = transaction;
            identityCheck.CommandText = "SELECT uidvalidity FROM folders WHERE id = $folder";
            identityCheck.Parameters.AddWithValue("$folder", folderId);
            var current = identityCheck.ExecuteScalar();
            if (current is null or DBNull || Convert.ToInt64(current) != validity)
                throw EngineException.Invalid("mailbox snapshot identity changed");
        }
        var rows = new List<(string Id, uint Uid, long Flags)>();
        using (var select = _conn.CreateCommand())
        {
            select.Transaction = transaction;
            select.CommandText = "SELECT id, uid, flags FROM messages WHERE folder_id = $folder";
            select.Parameters.AddWithValue("$folder", folderId);
            using var reader = select.ExecuteReader();
            while (reader.Read()) rows.Add((reader.GetString(0), checked((uint)reader.GetInt64(1)), reader.GetInt64(2)));
        }
        foreach (var (id, uid, originalFlags) in rows)
        {
            using var command = _conn.CreateCommand();
            command.Transaction = transaction;
            command.Parameters.AddWithValue("$id", id);
            if (effective.TryGetValue(uid, out var flags))
            {
                if (originalFlags == flags.ToBits()) continue;
                command.CommandText = "UPDATE messages SET flags = $flags WHERE id = $id";
                command.Parameters.AddWithValue("$flags", flags.ToBits());
            }
            else
            {
                command.CommandText = "DELETE FROM search_index WHERE message_id = $id; DELETE FROM messages WHERE id = $id";
            }
            command.ExecuteNonQuery();
        }
        using var counts = _conn.CreateCommand();
        counts.Transaction = transaction;
        counts.CommandText = """
            UPDATE folders SET total_count = (SELECT COUNT(*) FROM messages WHERE folder_id = $folder),
                unread_count = (SELECT COUNT(*) FROM messages WHERE folder_id = $folder AND (flags & 1) = 0)
            WHERE id = $folder
            """;
        counts.Parameters.AddWithValue("$folder", folderId);
        counts.ExecuteNonQuery();
        transaction.Commit();
    }

    public void PutBody(string messageId, string html, string text)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO bodies (message_id, html_sanitized, text, fetched_at)
            VALUES ($id, $html, $text, strftime('%s','now'))
            ON CONFLICT(message_id) DO UPDATE SET html_sanitized=excluded.html_sanitized, text=excluded.text, fetched_at=excluded.fetched_at
            """;
        cmd.Parameters.AddWithValue("$id", messageId);
        cmd.Parameters.AddWithValue("$html", html);
        cmd.Parameters.AddWithValue("$text", text);
        cmd.ExecuteNonQuery();
        using var state = _conn.CreateCommand();
        state.CommandText = "UPDATE messages SET body_state = 'full' WHERE id = $id";
        state.Parameters.AddWithValue("$id", messageId);
        state.ExecuteNonQuery();
    }

    public MessageBody? GetBody(string messageId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT html_sanitized, text FROM bodies WHERE message_id = $id";
        cmd.Parameters.AddWithValue("$id", messageId);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return new MessageBody(reader.IsDBNull(1) ? "" : reader.GetString(1), reader.IsDBNull(0) ? "" : reader.GetString(0));
    }

    public void EnqueueOp(string accountId, string type, string payload)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO ops_queue (account_id, type, payload_json, created_at, state)
            VALUES ($aid, $t, $p, strftime('%s','now'), 'pending')
            """;
        cmd.Parameters.AddWithValue("$aid", accountId);
        cmd.Parameters.AddWithValue("$t", type);
        cmd.Parameters.AddWithValue("$p", payload);
        cmd.ExecuteNonQuery();
    }

    public IReadOnlyList<(long Id, string AccountId, string Type, string Payload)> ListPendingOps()
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, account_id, type, payload_json FROM ops_queue WHERE state = 'pending' ORDER BY id";
        using var reader = cmd.ExecuteReader();
        var list = new List<(long, string, string, string)>();
        while (reader.Read())
        {
            list.Add((reader.GetInt64(0), reader.GetString(1), reader.GetString(2), reader.GetString(3)));
        }

        return list;
    }

    public void MarkOp(long id, string state)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "UPDATE ops_queue SET state = $s WHERE id = $id";
        cmd.Parameters.AddWithValue("$s", state);
        cmd.Parameters.AddWithValue("$id", id);
        cmd.ExecuteNonQuery();
    }

    public void InsertOutbox(string id, string accountId, string json, string state, long? sendAt = null)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO outbox (id, account_id, draft_json, scheduled_at, state, retry_count)
            VALUES ($id, $aid, $json, $at, $state, 0)
            """;
        cmd.Parameters.AddWithValue("$id", id);
        cmd.Parameters.AddWithValue("$aid", accountId);
        cmd.Parameters.AddWithValue("$json", json);
        cmd.Parameters.AddWithValue("$at", (object?)sendAt ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$state", state);
        cmd.ExecuteNonQuery();
    }

    public IReadOnlyList<string> DueOutbox(long nowUnix)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            SELECT id FROM outbox
            WHERE scheduled_at IS NOT NULL AND scheduled_at <= $now AND state IN ('scheduled', 'queued')
            """;
        cmd.Parameters.AddWithValue("$now", nowUnix);
        using var reader = cmd.ExecuteReader();
        var list = new List<string>();
        while (reader.Read())
        {
            list.Add(reader.GetString(0));
        }

        return list;
    }

    public (string AccountId, string Json, string State)? GetOutbox(string id)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT account_id, draft_json, state FROM outbox WHERE id = $id";
        cmd.Parameters.AddWithValue("$id", id);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return (reader.GetString(0), reader.GetString(1), reader.GetString(2));
    }

    public void SetOutboxState(string id, string state, string? error)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "UPDATE outbox SET state = $s, error = $e WHERE id = $id";
        cmd.Parameters.AddWithValue("$s", state);
        cmd.Parameters.AddWithValue("$e", (object?)error ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$id", id);
        cmd.ExecuteNonQuery();
    }

    public int ClearFailedQueue()
    {
        using var transaction = _conn.BeginTransaction();
        using var command = _conn.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = "DELETE FROM ops_queue WHERE state = 'failed'";
        var count = command.ExecuteNonQuery();
        command.CommandText = """
            UPDATE outbox SET state = 'draft', retry_count = 0, scheduled_at = NULL, error = NULL
            WHERE state = 'failed'
            """;
        count += command.ExecuteNonQuery();
        transaction.Commit();
        return count;
    }

    public string? DefaultSignature(string accountId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT body FROM signatures WHERE account_id = $id AND is_default = 1 LIMIT 1";
        cmd.Parameters.AddWithValue("$id", accountId);
        return cmd.ExecuteScalar() as string;
    }

    public Folder? FolderByRole(string accountId, string role)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, path, role, unread_count, total_count FROM folders WHERE account_id = $aid AND role = $role LIMIT 1";
        cmd.Parameters.AddWithValue("$aid", accountId);
        cmd.Parameters.AddWithValue("$role", role);
        using var reader = cmd.ExecuteReader();
        if (!reader.Read())
        {
            return null;
        }

        return new Folder(reader.GetString(0), reader.GetString(1), reader.GetString(2), (uint)reader.GetInt64(3), (uint)reader.GetInt64(4));
    }

    public void UpsertAttachment(string messageId, AttachmentHandle handle, byte[] data)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO attachments (id, message_id, name, mime, size, cid, state, data)
            VALUES ($id, $mid, $name, $mime, $size, $cid, 'local', $data)
            ON CONFLICT(id) DO UPDATE SET data=excluded.data, state='local'
            """;
        cmd.Parameters.AddWithValue("$id", handle.Id);
        cmd.Parameters.AddWithValue("$mid", messageId);
        cmd.Parameters.AddWithValue("$name", handle.Name);
        cmd.Parameters.AddWithValue("$mime", handle.Mime);
        cmd.Parameters.AddWithValue("$size", (long)handle.Size);
        cmd.Parameters.AddWithValue("$cid", (object?)handle.Cid ?? DBNull.Value);
        cmd.Parameters.AddWithValue("$data", data);
        cmd.ExecuteNonQuery();
    }

    public IReadOnlyList<AttachmentHandle> ListAttachments(string messageId)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = "SELECT id, name, mime, size, cid FROM attachments WHERE message_id = $id";
        cmd.Parameters.AddWithValue("$id", messageId);
        using var reader = cmd.ExecuteReader();
        var list = new List<AttachmentHandle>();
        while (reader.Read())
        {
            list.Add(new AttachmentHandle(
                reader.GetString(0),
                reader.IsDBNull(1) ? "attachment" : reader.GetString(1),
                reader.IsDBNull(2) ? "application/octet-stream" : reader.GetString(2),
                reader.IsDBNull(3) ? 0 : (ulong)reader.GetInt64(3),
                reader.IsDBNull(4) ? null : reader.GetString(4)));
        }

        return list;
    }

    private void IndexMessage(string id, string subject, string fromJson, string snippet)
    {
        using var del = _conn.CreateCommand();
        del.CommandText = "DELETE FROM search_index WHERE message_id = $id";
        del.Parameters.AddWithValue("$id", id);
        del.ExecuteNonQuery();
        using var ins = _conn.CreateCommand();
        ins.CommandText = "INSERT INTO search_index (message_id, subject, from_addr, body) VALUES ($id, $s, $f, $b)";
        ins.Parameters.AddWithValue("$id", id);
        ins.Parameters.AddWithValue("$s", subject);
        ins.Parameters.AddWithValue("$f", fromJson);
        ins.Parameters.AddWithValue("$b", snippet);
        ins.ExecuteNonQuery();
    }

    private static IReadOnlyList<MessageRow> ReadRows(SqliteCommand cmd)
    {
        using var reader = cmd.ExecuteReader();
        var list = new List<MessageRow>();
        while (reader.Read())
        {
            var fromJson = reader.IsDBNull(2) ? "[]" : reader.GetString(2);
            var from = ParseFrom(fromJson);
            var flags = MailFlags.FromBits(reader.IsDBNull(4) ? 0 : reader.GetInt64(4));
            list.Add(new MessageRow(
                reader.GetString(0),
                reader.IsDBNull(1) ? "" : reader.GetString(1),
                from,
                reader.IsDBNull(3) ? "" : reader.GetString(3),
                !flags.Seen,
                reader.IsDBNull(5) ? 0 : reader.GetInt64(5),
                !reader.IsDBNull(6) && reader.GetInt64(6) != 0,
                reader.IsDBNull(7) ? null : reader.GetString(7),
                ReadAddresses(reader, 8), ReadAddresses(reader, 9), ReadAddresses(reader, 10)));
        }

        return list;
    }

    private static IReadOnlyList<string> ReadAddresses(SqliteDataReader reader, int ordinal)
    {
        if (reader.IsDBNull(ordinal)) return [];
        try
        {
            using var document = JsonDocument.Parse(reader.GetString(ordinal));
            return document.RootElement.EnumerateArray().Select(item => item.ValueKind == JsonValueKind.String
                ? item.GetString() : item.TryGetProperty("email", out var email) ? email.GetString() : null)
                .Where(value => !string.IsNullOrWhiteSpace(value)).Select(value => value!).ToArray();
        }
        catch (JsonException) { return []; }
        catch (InvalidOperationException) { return []; }
    }

    private static string ParseFrom(string json)
    {
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(json);
            if (doc.RootElement.ValueKind == System.Text.Json.JsonValueKind.Array && doc.RootElement.GetArrayLength() > 0)
            {
                var first = doc.RootElement[0];
                if (first.TryGetProperty("email", out var email))
                {
                    return email.GetString() ?? "";
                }
            }
        }
        catch (System.Text.Json.JsonException)
        {
        }

        return json;
    }

    private void Migrate()
    {
        var version = UserVersion();
        if (version < 1)
        {
            Exec(V1);
            Exec("PRAGMA user_version = 1;");
        }

        if (version < 2)
        {
            Exec(V2);
            Exec("PRAGMA user_version = 2;");
        }

        if (version < 3)
        {
            Exec(V3);
            Exec("PRAGMA user_version = 3;");
        }
        if (version < 4)
        {
            using var migration = _conn.BeginTransaction();
            using var command = _conn.CreateCommand();
            command.Transaction = migration;
            command.CommandText = "ALTER TABLE messages ADD COLUMN reply_to_json TEXT; PRAGMA user_version = 4;";
            command.ExecuteNonQuery();
            migration.Commit();
        }
    }

    private void Exec(string sql)
    {
        using var cmd = _conn.CreateCommand();
        cmd.CommandText = sql;
        cmd.ExecuteNonQuery();
    }

    private const string V1 = """
        CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            display_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            mode TEXT NOT NULL,
            color TEXT NOT NULL,
            sync_scope TEXT NOT NULL,
            oauth_ref TEXT,
            api_token_ref TEXT,
            quirks TEXT,
            state TEXT NOT NULL DEFAULT 'offline',
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS folders (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            remote_name TEXT NOT NULL,
            path TEXT NOT NULL,
            role TEXT NOT NULL,
            uidvalidity INTEGER,
            highestmodseq INTEGER,
            unread_count INTEGER NOT NULL DEFAULT 0,
            total_count INTEGER NOT NULL DEFAULT 0,
            sync_cursor TEXT
        );
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            uid INTEGER,
            message_id TEXT,
            in_reply_to TEXT,
            thread_id TEXT,
            subject TEXT,
            from_json TEXT,
            to_json TEXT,
            cc_json TEXT,
            date INTEGER,
            snippet TEXT,
            size INTEGER,
            flags INTEGER NOT NULL DEFAULT 0,
            labels_json TEXT,
            has_attachments INTEGER NOT NULL DEFAULT 0,
            body_state TEXT NOT NULL DEFAULT 'headers',
            UNIQUE(account_id, folder_id, uid)
        );
        CREATE TABLE IF NOT EXISTS bodies (
            message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
            html_sanitized TEXT,
            text TEXT,
            fetched_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            name TEXT,
            mime TEXT,
            size INTEGER,
            cid TEXT,
            local_path TEXT,
            state TEXT NOT NULL DEFAULT 'remote'
        );
        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            subject_norm TEXT,
            message_count INTEGER NOT NULL DEFAULT 0,
            last_date INTEGER
        );
        CREATE TABLE IF NOT EXISTS outbox (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            draft_json TEXT NOT NULL,
            scheduled_at INTEGER,
            state TEXT NOT NULL,
            retry_count INTEGER NOT NULL DEFAULT 0,
            error TEXT
        );
        CREATE TABLE IF NOT EXISTS ops_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            state TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS rules (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            definition_json TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS contacts (
            id TEXT PRIMARY KEY,
            account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
            email TEXT NOT NULL,
            name TEXT,
            vip INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
            message_id UNINDEXED,
            subject,
            from_addr,
            body,
            tokenize = 'unicode61'
        );
        """;

    private const string V2 = """
        ALTER TABLE messages ADD COLUMN snooze_until INTEGER;
        CREATE TABLE IF NOT EXISTS signatures (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            body TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0
        );
        """;

    private const string V3 = """
        ALTER TABLE attachments ADD COLUMN data BLOB;
        """;
}
