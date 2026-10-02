//! SQLite 存储。schema 版本化，六端同一迁移脚本。

mod schema;

use chck_types::{
    Account, AccountId, AccountMode, AccountState, Address, AttachmentHandle, AttachmentId,
    Contact, EngineError, Flags, Folder, FolderId, FolderRole, MessageBody, MessageId,
    MessageSummary, Page, Rule, SearchHit, SearchQuery, SearchSource, Signature, SyncScope,
    ThreadId,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const SCHEMA_VERSION: i32 = 4;

#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub id: String,
    pub account_id: AccountId,
    pub state: String,
    pub retry_count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub uid: u32,
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub date_unix: i64,
    pub snippet: String,
    pub flags: Flags,
    pub has_attachments: bool,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub thread_id: Option<String>,
    pub size: u32,
}

/// Repair old cached RFC 2047 headers once, without clearing messages or sync cursors.
fn repair_encoded_headers(conn: &Connection) -> Result<(), EngineError> {
    let repaired: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key = 'headers-rfc2047-v1')",
            [],
            |r| r.get(0),
        )
        .map_err(storage_err)?;
    if repaired {
        return Ok(());
    }
    let tx = conn.unchecked_transaction().map_err(storage_err)?;
    {
        let mut stmt = tx.prepare("SELECT id, COALESCE(subject, ''), COALESCE(from_json, '[]'), COALESCE(snippet, '') FROM messages WHERE subject LIKE '%=?%' OR from_json LIKE '%=?%'").map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(storage_err)?;
        for row in rows {
            let (id, subject, from_json, snippet) = row.map_err(storage_err)?;
            let decoded = chck_mime::decode_header(&subject);
            let mut from: Vec<Address> = serde_json::from_str(&from_json)
                .map_err(|e| EngineError::Storage(e.to_string()))?;
            for address in &mut from {
                address.name = address.name.as_ref().map(|name| chck_mime::decode_header(name));
            }
            let snippet = if subject.starts_with(&snippet) {
                decoded.chars().take(140).collect()
            } else {
                snippet
            };
            let from_json =
                serde_json::to_string(&from).map_err(|e| EngineError::Storage(e.to_string()))?;
            tx.execute(
                "UPDATE messages SET subject = ?1, from_json = ?2, snippet = ?3 WHERE id = ?4",
                params![decoded, from_json, snippet, id],
            )
            .map_err(storage_err)?;
            // Preserve any full body already indexed by the reader.
            tx.execute(
                "UPDATE search_index SET subject = ?1 WHERE message_id = ?2",
                params![decoded, id],
            )
            .map_err(storage_err)?;
        }
    }
    tx.execute("INSERT INTO settings (key, value) VALUES ('headers-rfc2047-v1', '1')", [])
        .map_err(storage_err)?;
    tx.commit().map_err(storage_err)
}

pub struct Store {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(&path).map_err(storage_err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(storage_err)?;
        schema::migrate(&conn)?;
        repair_encoded_headers(&conn)?;
        Ok(Self { conn: Mutex::new(conn), path })
    }

    pub fn open_in_memory() -> Result<Self, EngineError> {
        let conn = Connection::open_in_memory().map_err(storage_err)?;
        conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(storage_err)?;
        schema::migrate(&conn)?;
        repair_encoded_headers(&conn)?;
        Ok(Self { conn: Mutex::new(conn), path: PathBuf::from(":memory:") })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<i32, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let v: i32 =
            conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(storage_err)?;
        Ok(v)
    }

    pub fn insert_account(&self, account: &Account) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO accounts (id, email, display_name, provider, mode, color, sync_scope, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%s','now'))",
            params![
                account.id.as_str(),
                account.email,
                account.display_name,
                account.provider_id,
                mode_str(account.mode),
                account.color,
                scope_str(account.sync_scope),
                state_str(account.state),
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, email, display_name, provider, mode, color, sync_scope, state FROM accounts ORDER BY created_at",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Account {
                    id: AccountId::from(row.get::<_, String>(0)?),
                    email: row.get(1)?,
                    display_name: row.get(2)?,
                    provider_id: row.get(3)?,
                    mode: parse_mode(&row.get::<_, String>(4)?),
                    color: row.get(5)?,
                    sync_scope: parse_scope(&row.get::<_, String>(6)?),
                    state: parse_state(&row.get::<_, String>(7)?),
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn get_account(&self, id: &AccountId) -> Result<Option<Account>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT id, email, display_name, provider, mode, color, sync_scope, state FROM accounts WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok(Account {
                    id: AccountId::from(row.get::<_, String>(0)?),
                    email: row.get(1)?,
                    display_name: row.get(2)?,
                    provider_id: row.get(3)?,
                    mode: parse_mode(&row.get::<_, String>(4)?),
                    color: row.get(5)?,
                    sync_scope: parse_scope(&row.get::<_, String>(6)?),
                    state: parse_state(&row.get::<_, String>(7)?),
                })
            },
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn update_account(&self, account: &Account) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let n = conn
            .execute(
                "UPDATE accounts SET email=?2, display_name=?3, provider=?4, mode=?5, color=?6, sync_scope=?7, state=?8
                 WHERE id=?1",
                params![
                    account.id.as_str(),
                    account.email,
                    account.display_name,
                    account.provider_id,
                    mode_str(account.mode),
                    account.color,
                    scope_str(account.sync_scope),
                    state_str(account.state),
                ],
            )
            .map_err(storage_err)?;
        if n == 0 {
            return Err(EngineError::NotFound);
        }
        Ok(())
    }

    pub fn delete_account(&self, id: &AccountId) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("DELETE FROM accounts WHERE id = ?1", [id.as_str()]).map_err(storage_err)?;
        Ok(())
    }

    pub fn enqueue_op(
        &self,
        account_id: &AccountId,
        op_type: &str,
        payload_json: &str,
    ) -> Result<i64, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO ops_queue (account_id, type, payload_json, created_at, state)
             VALUES (?1, ?2, ?3, strftime('%s','now'), 'pending')",
            params![account_id.as_str(), op_type, payload_json],
        )
        .map_err(storage_err)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn set_kv(&self, key: &str, value: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn get_kv(&self, key: &str) -> Result<Option<String>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .map_err(storage_err)
    }

    pub fn delete_kv(&self, key: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("DELETE FROM settings WHERE key = ?1", [key]).map_err(storage_err)?;
        Ok(())
    }

    pub fn pending_ops_count(&self) -> Result<u32, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let n: u32 = conn
            .query_row("SELECT COUNT(*) FROM ops_queue WHERE state = 'pending'", [], |r| r.get(0))
            .map_err(storage_err)?;
        Ok(n)
    }

    pub fn upsert_folder(&self, folder: &Folder) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO folders (id, account_id, remote_name, path, role, unread_count, total_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                remote_name=excluded.remote_name,
                path=excluded.path,
                role=excluded.role,
                unread_count=excluded.unread_count,
                total_count=excluded.total_count",
            params![
                folder.id.as_str(),
                folder.account_id.as_str(),
                folder.remote_name,
                folder.path,
                folder.role.as_str(),
                folder.unread_count,
                folder.total_count,
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn list_folders(&self, account_id: &AccountId) -> Result<Vec<Folder>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, remote_name, path, role, unread_count, total_count
                 FROM folders WHERE account_id = ?1 ORDER BY path",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([account_id.as_str()], |row| {
                Ok(Folder {
                    id: FolderId::from(row.get::<_, String>(0)?),
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    remote_name: row.get(2)?,
                    path: row.get(3)?,
                    role: FolderRole::parse(&row.get::<_, String>(4)?),
                    unread_count: row.get(5)?,
                    total_count: row.get(6)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn get_folder(&self, id: &FolderId) -> Result<Option<Folder>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT id, account_id, remote_name, path, role, unread_count, total_count
             FROM folders WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok(Folder {
                    id: FolderId::from(row.get::<_, String>(0)?),
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    remote_name: row.get(2)?,
                    path: row.get(3)?,
                    role: FolderRole::parse(&row.get::<_, String>(4)?),
                    unread_count: row.get(5)?,
                    total_count: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn folder_cursor(&self, id: &FolderId) -> Result<u32, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let raw: Option<String> = conn
            .query_row("SELECT sync_cursor FROM folders WHERE id = ?1", [id.as_str()], |r| r.get(0))
            .optional()
            .map_err(storage_err)?
            .flatten();
        Ok(raw.and_then(|s| s.parse().ok()).unwrap_or(1))
    }

    pub fn set_folder_cursor(
        &self,
        id: &FolderId,
        uid: u32,
        total: u32,
        unread: u32,
    ) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "UPDATE folders SET sync_cursor = ?1, total_count = ?2, unread_count = ?3 WHERE id = ?4",
            params![uid.to_string(), total, unread, id.as_str()],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn upsert_message(
        &self,
        folder: &Folder,
        rec: &MessageRecord,
    ) -> Result<MessageId, EngineError> {
        let id = MessageId::from(format!("{}:{}", folder.id.as_str(), rec.uid));
        let from_json = serde_json::to_string(&rec.from).unwrap_or_else(|_| "[]".into());
        let to_json = serde_json::to_string(&rec.to).unwrap_or_else(|_| "[]".into());
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO messages (
                id, account_id, folder_id, uid, message_id, in_reply_to, thread_id, subject,
                from_json, to_json, date, snippet, size, flags, has_attachments, body_state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'headers')
             ON CONFLICT(account_id, folder_id, uid) DO UPDATE SET
                subject=excluded.subject,
                from_json=excluded.from_json,
                to_json=excluded.to_json,
                date=excluded.date,
                snippet=excluded.snippet,
                flags=excluded.flags,
                has_attachments=excluded.has_attachments,
                message_id=excluded.message_id,
                thread_id=excluded.thread_id,
                in_reply_to=excluded.in_reply_to",
            params![
                id.as_str(),
                folder.account_id.as_str(),
                folder.id.as_str(),
                rec.uid,
                rec.message_id,
                rec.in_reply_to,
                rec.thread_id,
                rec.subject,
                from_json,
                to_json,
                rec.date_unix,
                rec.snippet,
                rec.size,
                rec.flags.to_bits(),
                i64::from(rec.has_attachments),
            ],
        )
        .map_err(storage_err)?;
        let from_addr = rec.from.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(" ");
        index_message(&conn, id.as_str(), &rec.subject, &from_addr, &rec.snippet)?;
        Ok(id)
    }

    pub fn list_messages(
        &self,
        folder_id: &FolderId,
        page: Page,
    ) -> Result<Vec<MessageSummary>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, folder_id, subject, from_json, date, snippet, flags, has_attachments, thread_id,
                        COALESCE(to_json, '[]'), COALESCE(labels_json, '[]')
                 FROM messages
                 WHERE folder_id = ?1
                   AND (snooze_until IS NULL OR snooze_until <= CAST(strftime('%s','now') AS INTEGER))
                 ORDER BY date DESC, uid DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map(params![folder_id.as_str(), page.limit, page.offset], map_summary)
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn unified_inbox(&self, page: Page) -> Result<Vec<MessageSummary>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.account_id, m.folder_id, m.subject, m.from_json, m.date, m.snippet, m.flags, m.has_attachments, m.thread_id,
                        COALESCE(m.to_json, '[]'), COALESCE(m.labels_json, '[]')
                 FROM messages m
                 JOIN folders f ON f.id = m.folder_id
                 WHERE f.role = 'inbox'
                   AND (m.snooze_until IS NULL OR m.snooze_until <= CAST(strftime('%s','now') AS INTEGER))
                 ORDER BY m.date DESC, m.uid DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(storage_err)?;
        let rows =
            stmt.query_map(params![page.limit, page.offset], map_summary).map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn list_thread(&self, thread_id: &ThreadId) -> Result<Vec<MessageSummary>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, folder_id, subject, from_json, date, snippet, flags, has_attachments, thread_id,
                        COALESCE(to_json, '[]'), COALESCE(labels_json, '[]')
                 FROM messages WHERE thread_id = ?1 ORDER BY date ASC, uid ASC",
            )
            .map_err(storage_err)?;
        let rows = stmt.query_map([thread_id.as_str()], map_summary).map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn find_thread_id(
        &self,
        account_id: &AccountId,
        in_reply_to: Option<&str>,
        message_id: Option<&str>,
    ) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        if let Some(parent) = in_reply_to.filter(|s| !s.is_empty())
            && let Ok(Some(t)) = conn.query_row(
                "SELECT thread_id FROM messages WHERE account_id = ?1 AND (message_id = ?2 OR thread_id = ?2) AND thread_id IS NOT NULL LIMIT 1",
                params![account_id.as_str(), parent],
                |r| r.get::<_, Option<String>>(0),
            )
        {
            return Some(t);
        }
        message_id.filter(|s| !s.is_empty()).map(ToString::to_string)
    }

    pub fn get_message_meta(
        &self,
        id: &MessageId,
    ) -> Result<Option<(AccountId, FolderId, u32, Folder)>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT m.account_id, m.folder_id, m.uid, f.remote_name, f.path, f.role, f.unread_count, f.total_count
             FROM messages m JOIN folders f ON f.id = m.folder_id WHERE m.id = ?1",
            [id.as_str()],
            |row| {
                let account_id = AccountId::from(row.get::<_, String>(0)?);
                let folder_id = FolderId::from(row.get::<_, String>(1)?);
                let uid: u32 = row.get(2)?;
                let folder = Folder {
                    id: folder_id.clone(),
                    account_id: account_id.clone(),
                    remote_name: row.get(3)?,
                    path: row.get(4)?,
                    role: FolderRole::parse(&row.get::<_, String>(5)?),
                    unread_count: row.get(6)?,
                    total_count: row.get(7)?,
                };
                Ok((account_id, folder_id, uid, folder))
            },
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn get_body(&self, id: &MessageId) -> Result<Option<MessageBody>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT html_sanitized, text, remote_blocked FROM bodies WHERE message_id = ?1",
            [id.as_str()],
            |row| {
                Ok(MessageBody {
                    id: id.clone(),
                    html_sanitized: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    text: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    remote_blocked: row.get::<_, u32>(2)?,
                })
            },
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn upsert_attachment(
        &self,
        message_id: &MessageId,
        handle: &AttachmentHandle,
        data: &[u8],
    ) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO attachments (id, message_id, name, mime, size, cid, state, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'local', ?7)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, mime=excluded.mime, size=excluded.size,
                cid=excluded.cid, state='local', data=excluded.data",
            params![
                handle.id.as_str(),
                message_id.as_str(),
                handle.name,
                handle.mime,
                handle.size as i64,
                handle.cid,
                data,
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn list_attachments(
        &self,
        message_id: &MessageId,
    ) -> Result<Vec<AttachmentHandle>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, mime, size, cid FROM attachments WHERE message_id = ?1 ORDER BY name",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([message_id.as_str()], |row| {
                Ok(AttachmentHandle {
                    id: AttachmentId::from(row.get::<_, String>(0)?),
                    name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    mime: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    size: row.get::<_, i64>(3)? as u64,
                    cid: row.get(4)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn attachment_data(&self, id: &AttachmentId) -> Result<Option<Vec<u8>>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row("SELECT data FROM attachments WHERE id = ?1", [id.as_str()], |row| {
            row.get::<_, Option<Vec<u8>>>(0)
        })
        .optional()
        .map_err(storage_err)
        .map(|o| o.flatten())
    }

    pub fn put_body(&self, body: &MessageBody) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO bodies (message_id, html_sanitized, text, remote_blocked, fetched_at)
             VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))
             ON CONFLICT(message_id) DO UPDATE SET
                html_sanitized=excluded.html_sanitized,
                text=excluded.text,
                remote_blocked=excluded.remote_blocked,
                fetched_at=excluded.fetched_at",
            params![body.id.as_str(), body.html_sanitized, body.text, body.remote_blocked],
        )
        .map_err(storage_err)?;
        conn.execute("UPDATE messages SET body_state = 'full' WHERE id = ?1", [body.id.as_str()])
            .map_err(storage_err)?;
        let (subject, from_json): (String, String) = conn
            .query_row(
                "SELECT COALESCE(subject,''), COALESCE(from_json,'[]') FROM messages WHERE id = ?1",
                [body.id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(storage_err)?;
        let from: Vec<Address> = serde_json::from_str(&from_json).unwrap_or_default();
        let from_addr = from.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(" ");
        index_message(&conn, body.id.as_str(), &subject, &from_addr, &body.text)?;
        Ok(())
    }

    pub fn set_message_flags(&self, id: &MessageId, flags: Flags) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "UPDATE messages SET flags = ?1 WHERE id = ?2",
            params![flags.to_bits(), id.as_str()],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn delete_message(&self, id: &MessageId) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("DELETE FROM search_index WHERE message_id = ?1", [id.as_str()])
            .map_err(storage_err)?;
        conn.execute("DELETE FROM messages WHERE id = ?1", [id.as_str()]).map_err(storage_err)?;
        Ok(())
    }

    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, EngineError> {
        let text = query.text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let match_q = fts_quote(text);
        let acc = query.account_id.as_ref().map(AccountId::as_str);
        let folder = query.folder_id.as_ref().map(FolderId::as_str);
        let mut stmt = conn
            .prepare(
                "SELECT search_index.message_id FROM search_index
                 JOIN messages m ON m.id = search_index.message_id
                 WHERE search_index MATCH ?1
                   AND (?2 IS NULL OR m.account_id = ?2)
                   AND (?3 IS NULL OR m.folder_id = ?3)
                   AND (?4 = 0 OR m.has_attachments = 1)
                   AND (m.snooze_until IS NULL OR m.snooze_until <= CAST(strftime('%s','now') AS INTEGER))
                 ORDER BY rank
                 LIMIT 50",
            )
            .map_err(storage_err)?;
        let attach: i64 = i64::from(query.has_attachment == Some(true));
        let rows = stmt
            .query_map(params![match_q, acc, folder, attach], |row| {
                Ok(SearchHit {
                    message_id: MessageId::from(row.get::<_, String>(0)?),
                    source: SearchSource::Local,
                    rank: 0,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn move_message(
        &self,
        id: &MessageId,
        dest: &Folder,
        destination_uid: u32,
    ) -> Result<(), EngineError> {
        let new_id = format!("{}:{destination_uid}", dest.id.as_str());
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let tx = conn.unchecked_transaction().map_err(storage_err)?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON;").map_err(storage_err)?;
        let updated = tx.execute(
            "UPDATE messages SET id = ?1, folder_id = ?2, uid = ?3 WHERE id = ?4 AND account_id = ?5",
            params![new_id, dest.id.as_str(), destination_uid, id.as_str(), dest.account_id.as_str()],
        ).map_err(storage_err)?;
        if updated != 1 {
            return Err(EngineError::NotFound);
        }
        for table in ["bodies", "attachments", "search_index"] {
            tx.execute(
                &format!("UPDATE {table} SET message_id = ?1 WHERE message_id = ?2"),
                params![new_id, id.as_str()],
            )
            .map_err(storage_err)?;
        }
        tx.commit().map_err(storage_err)
    }

    pub fn folder_uidvalidity(&self, id: &FolderId) -> Result<Option<u32>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row("SELECT uidvalidity FROM folders WHERE id = ?1", [id.as_str()], |r| r.get(0))
            .map_err(storage_err)
    }

    pub fn set_folder_uidvalidity(&self, id: &FolderId, validity: u32) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let tx = conn.unchecked_transaction().map_err(storage_err)?;
        let previous: Option<u32> = tx
            .query_row("SELECT uidvalidity FROM folders WHERE id = ?1", [id.as_str()], |r| r.get(0))
            .map_err(storage_err)?;
        if previous.is_some_and(|old| old != validity) {
            tx.execute("DELETE FROM search_index WHERE message_id IN (SELECT id FROM messages WHERE folder_id = ?1)", [id.as_str()]).map_err(storage_err)?;
            tx.execute("DELETE FROM messages WHERE folder_id = ?1", [id.as_str()])
                .map_err(storage_err)?;
            tx.execute("UPDATE folders SET sync_cursor = NULL WHERE id = ?1", [id.as_str()])
                .map_err(storage_err)?;
        }
        tx.execute(
            "UPDATE folders SET uidvalidity = ?1 WHERE id = ?2",
            params![validity, id.as_str()],
        )
        .map_err(storage_err)?;
        tx.commit().map_err(storage_err)
    }

    pub fn claim_move(&self, id: &MessageId, dest: &FolderId) -> Result<bool, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO settings(key, value) VALUES (?1, ?2)",
                params![format!("move:{}", id.as_str()), dest.as_str()],
            )
            .map_err(storage_err)?;
        Ok(changed == 1)
    }

    pub fn list_pending_ops(&self) -> Result<Vec<(i64, AccountId, String, String)>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare("SELECT id, account_id, type, payload_json FROM ops_queue WHERE state = 'pending' ORDER BY id")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    AccountId::from(row.get::<_, String>(1)?),
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn mark_op(&self, id: i64, state: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("UPDATE ops_queue SET state = ?1 WHERE id = ?2", params![state, id])
            .map_err(storage_err)?;
        Ok(())
    }

    pub fn insert_outbox(
        &self,
        draft_id: &str,
        account_id: &AccountId,
        draft_json: &str,
        state: &str,
    ) -> Result<(), EngineError> {
        self.insert_outbox_at(draft_id, account_id, draft_json, state, None)
    }

    pub fn insert_outbox_at(
        &self,
        draft_id: &str,
        account_id: &AccountId,
        draft_json: &str,
        state: &str,
        scheduled_at: Option<i64>,
    ) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO outbox (id, account_id, draft_json, state, retry_count, scheduled_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)
             ON CONFLICT(id) DO UPDATE SET draft_json=excluded.draft_json, state=excluded.state, scheduled_at=excluded.scheduled_at",
            params![draft_id, account_id.as_str(), draft_json, state, scheduled_at],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn retry_failed_sends(&self) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("UPDATE outbox SET state = 'queued', retry_count = 0, error = NULL WHERE state = 'failed'", []).map_err(storage_err)?;
        Ok(())
    }

    /// Remove failed operations and return failed sends to inert drafts, retaining
    /// the only stored copy of their composed message. Never schedules a retry.
    pub fn clear_failed_queue(&self) -> Result<(usize, usize), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let tx = conn.unchecked_transaction().map_err(storage_err)?;
        let ops =
            tx.execute("DELETE FROM ops_queue WHERE state = 'failed'", []).map_err(storage_err)?;
        let outbox = tx.execute("UPDATE outbox SET state = 'draft', retry_count = 0, scheduled_at = NULL, error = NULL WHERE state = 'failed'", []).map_err(storage_err)?;
        tx.commit().map_err(storage_err)?;
        Ok((ops, outbox))
    }

    pub fn due_outbox(&self, now_unix: i64) -> Result<Vec<String>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id FROM outbox WHERE
                    (scheduled_at IS NOT NULL AND scheduled_at <= ?1 AND state IN ('scheduled', 'queued'))
                    OR (state = 'failed' AND retry_count < 3)
                    OR (state = 'queued' AND scheduled_at IS NULL)",
            )
            .map_err(storage_err)?;
        let rows = stmt.query_map([now_unix], |r| r.get::<_, String>(0)).map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn upsert_rule(&self, rule: &Rule) -> Result<(), EngineError> {
        let def = serde_json::to_string(&rule.definition)
            .map_err(|e| EngineError::Storage(e.to_string()))?;
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO rules (id, account_id, name, definition_json, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, definition_json=excluded.definition_json, enabled=excluded.enabled",
            params![
                rule.id,
                rule.account_id.as_str(),
                rule.name,
                def,
                i64::from(rule.enabled),
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn list_rules(&self, account_id: &AccountId) -> Result<Vec<Rule>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare("SELECT id, account_id, name, definition_json, enabled FROM rules WHERE account_id = ?1")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([account_id.as_str()], |row| {
                let def_json: String = row.get(3)?;
                let definition = serde_json::from_str(&def_json).unwrap_or_default();
                Ok(Rule {
                    id: row.get(0)?,
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                    definition,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn delete_rule(&self, rule_id: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("DELETE FROM rules WHERE id = ?1", [rule_id]).map_err(storage_err)?;
        Ok(())
    }

    pub fn upsert_signature(&self, sig: &Signature) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        if sig.is_default {
            conn.execute(
                "UPDATE signatures SET is_default = 0 WHERE account_id = ?1",
                [sig.account_id.as_str()],
            )
            .map_err(storage_err)?;
        }
        conn.execute(
            "INSERT INTO signatures (id, account_id, name, body, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, body=excluded.body, is_default=excluded.is_default",
            params![
                sig.id,
                sig.account_id.as_str(),
                sig.name,
                sig.body,
                i64::from(sig.is_default),
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn list_signatures(&self, account_id: &AccountId) -> Result<Vec<Signature>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, name, body, is_default FROM signatures WHERE account_id = ?1 ORDER BY name",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([account_id.as_str()], |row| {
                Ok(Signature {
                    id: row.get(0)?,
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    body: row.get(3)?,
                    is_default: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn default_signature(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<Signature>, EngineError> {
        Ok(self.list_signatures(account_id)?.into_iter().find(|s| s.is_default))
    }

    pub fn delete_signature(&self, signature_id: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute("DELETE FROM signatures WHERE id = ?1", [signature_id])
            .map_err(storage_err)?;
        Ok(())
    }

    pub fn upsert_contact(&self, c: &Contact) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "INSERT INTO contacts (id, account_id, email, name, vip)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET email=excluded.email, name=excluded.name, vip=excluded.vip, account_id=excluded.account_id",
            params![
                c.id,
                c.account_id.as_ref().map(AccountId::as_str),
                c.email.to_ascii_lowercase(),
                c.name,
                i64::from(c.vip),
            ],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn find_contact_email(&self, email: &str) -> Result<Option<Contact>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT id, account_id, email, name, vip FROM contacts WHERE lower(email) = lower(?1) LIMIT 1",
            [email],
            map_contact,
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn list_contacts(
        &self,
        account_id: Option<&AccountId>,
    ) -> Result<Vec<Contact>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, email, name, vip FROM contacts
                 WHERE ?1 IS NULL OR account_id = ?1 OR account_id IS NULL
                 ORDER BY vip DESC, name, email",
            )
            .map_err(storage_err)?;
        let acc = account_id.map(AccountId::as_str);
        let rows = stmt.query_map([acc], map_contact).map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn set_vip(&self, contact_id: &str, vip: bool) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "UPDATE contacts SET vip = ?1 WHERE id = ?2",
            params![i64::from(vip), contact_id],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn export_accounts_json(&self) -> Result<String, EngineError> {
        let accounts = self.list_accounts()?;
        serde_json::to_string(&serde_json::json!({ "accounts": accounts, "version": 1 }))
            .map_err(|e| EngineError::Storage(e.to_string()))
    }

    pub fn list_all_rules(&self) -> Result<Vec<Rule>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare("SELECT id, account_id, name, definition_json, enabled FROM rules")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                let def_json: String = row.get(3)?;
                let definition = serde_json::from_str(&def_json).unwrap_or_default();
                Ok(Rule {
                    id: row.get(0)?,
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                    definition,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn list_all_signatures(&self) -> Result<Vec<Signature>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare("SELECT id, account_id, name, body, is_default FROM signatures")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Signature {
                    id: row.get(0)?,
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    body: row.get(3)?,
                    is_default: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn add_label(&self, id: &MessageId, label: &str) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let raw: Option<String> = conn
            .query_row("SELECT labels_json FROM messages WHERE id = ?1", [id.as_str()], |r| {
                r.get(0)
            })
            .optional()
            .map_err(storage_err)?
            .flatten();
        let mut labels: Vec<String> =
            raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if !labels.iter().any(|existing| existing.eq_ignore_ascii_case(label)) {
            labels.push(label.to_string());
        }
        let json = serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "UPDATE messages SET labels_json = ?1 WHERE id = ?2",
            params![json, id.as_str()],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn set_snooze(&self, id: &MessageId, until: Option<i64>) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "UPDATE messages SET snooze_until = ?1 WHERE id = ?2",
            params![until, id.as_str()],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn due_snoozes(&self, now_unix: i64) -> Result<Vec<MessageId>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id FROM messages WHERE snooze_until IS NOT NULL AND snooze_until <= ?1",
            )
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([now_unix], |r| Ok(MessageId::from(r.get::<_, String>(0)?)))
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn summary(&self, id: &MessageId) -> Result<Option<MessageSummary>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT id, account_id, folder_id, subject, from_json, date, snippet, flags, has_attachments, thread_id,
                    COALESCE(to_json, '[]'), COALESCE(labels_json, '[]')
             FROM messages WHERE id = ?1",
            [id.as_str()],
            map_summary,
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn folder_by_path(
        &self,
        account_id: &AccountId,
        path: &str,
    ) -> Result<Option<Folder>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT id, account_id, remote_name, path, role, unread_count, total_count FROM folders WHERE account_id = ?1 AND (path = ?2 OR remote_name = ?2)",
            params![account_id.as_str(), path],
            |row| {
                Ok(Folder {
                    id: FolderId::from(row.get::<_, String>(0)?),
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    remote_name: row.get(2)?,
                    path: row.get(3)?,
                    role: FolderRole::parse(&row.get::<_, String>(4)?),
                    unread_count: row.get(5)?,
                    total_count: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn get_outbox(
        &self,
        draft_id: &str,
    ) -> Result<Option<(AccountId, String, String)>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.query_row(
            "SELECT account_id, draft_json, state FROM outbox WHERE id = ?1",
            [draft_id],
            |row| Ok((AccountId::from(row.get::<_, String>(0)?), row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(storage_err)
    }

    pub fn list_outbox(&self) -> Result<Vec<OutboxRow>, EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut stmt = conn
            .prepare("SELECT id, account_id, state, retry_count, error FROM outbox ORDER BY id")
            .map_err(storage_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(OutboxRow {
                    id: row.get(0)?,
                    account_id: AccountId::from(row.get::<_, String>(1)?),
                    state: row.get(2)?,
                    retry_count: row.get(3)?,
                    error: row.get(4)?,
                })
            })
            .map_err(storage_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn set_outbox_state(
        &self,
        draft_id: &str,
        state: &str,
        error: Option<&str>,
    ) -> Result<(), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        conn.execute(
            "UPDATE outbox SET state = ?1, error = ?2, retry_count = CASE WHEN ?1 = 'failed' THEN retry_count + 1 ELSE retry_count END WHERE id = ?3",
            params![state, error, draft_id],
        )
        .map_err(storage_err)?;
        Ok(())
    }

    pub fn message_count(&self, folder_id: &FolderId) -> Result<(u32, u32), EngineError> {
        let conn = self.conn.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let total: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE folder_id = ?1
                 AND (snooze_until IS NULL OR snooze_until <= CAST(strftime('%s','now') AS INTEGER))",
                [folder_id.as_str()],
                |r| r.get(0),
            )
            .map_err(storage_err)?;
        let unread: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE folder_id = ?1 AND (flags & 1) = 0
                 AND (snooze_until IS NULL OR snooze_until <= CAST(strftime('%s','now') AS INTEGER))",
                [folder_id.as_str()],
                |r| r.get(0),
            )
            .map_err(storage_err)?;
        Ok((total, unread))
    }
}

fn map_contact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get(0)?,
        account_id: row.get::<_, Option<String>>(1)?.map(AccountId::from),
        email: row.get(2)?,
        name: row.get(3)?,
        vip: row.get::<_, i64>(4)? != 0,
    })
}

fn map_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageSummary> {
    let from_json: String = row.get(4)?;
    let from: Vec<Address> = serde_json::from_str(&from_json).unwrap_or_default();
    let thread: Option<String> = row.get(9)?;
    Ok(MessageSummary {
        id: MessageId::from(row.get::<_, String>(0)?),
        account_id: AccountId::from(row.get::<_, String>(1)?),
        folder_id: FolderId::from(row.get::<_, String>(2)?),
        thread_id: thread.map(ThreadId::from),
        subject: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        from,
        date_unix: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
        snippet: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        flags: Flags::from_bits(row.get(7)?),
        has_attachments: row.get::<_, i64>(8)? != 0,
        to: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
        labels: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
    })
}

fn index_message(
    conn: &rusqlite::Connection,
    id: &str,
    subject: &str,
    from_addr: &str,
    body: &str,
) -> Result<(), EngineError> {
    conn.execute("DELETE FROM search_index WHERE message_id = ?1", [id]).map_err(storage_err)?;
    conn.execute(
        "INSERT INTO search_index (message_id, subject, from_addr, body) VALUES (?1, ?2, ?3, ?4)",
        params![id, subject, from_addr, body],
    )
    .map_err(storage_err)?;
    Ok(())
}

fn fts_quote(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '@' || *c == '.')
        .collect();
    let token = cleaned.split_whitespace().next().unwrap_or("");
    if token.is_empty() { "\"\"".into() } else { format!("\"{token}\"") }
}

fn storage_err(err: rusqlite::Error) -> EngineError {
    EngineError::Storage(err.to_string())
}

fn mode_str(mode: AccountMode) -> &'static str {
    match mode {
        AccountMode::Standard => "standard",
        AccountMode::Enhanced => "enhanced",
    }
}

fn parse_mode(s: &str) -> AccountMode {
    if s == "enhanced" { AccountMode::Enhanced } else { AccountMode::Standard }
}

fn state_str(state: AccountState) -> &'static str {
    match state {
        AccountState::Online => "online",
        AccountState::Syncing => "syncing",
        AccountState::Error => "error",
        AccountState::AuthRequired => "authRequired",
        AccountState::Offline => "offline",
    }
}

fn parse_state(s: &str) -> AccountState {
    match s {
        "syncing" => AccountState::Syncing,
        "error" => AccountState::Error,
        "authRequired" => AccountState::AuthRequired,
        "offline" => AccountState::Offline,
        _ => AccountState::Online,
    }
}

fn scope_str(scope: SyncScope) -> String {
    match scope {
        SyncScope::All => "all".into(),
        SyncScope::Last30Days => "last30".into(),
        SyncScope::HeadersOnly { limit } => format!("headers:{limit}"),
    }
}

fn parse_scope(s: &str) -> SyncScope {
    match s {
        "all" => SyncScope::All,
        x if x.starts_with("headers:") => {
            let limit = x.rsplit(':').next().and_then(|n| n.parse().ok()).unwrap_or(200);
            SyncScope::HeadersOnly { limit }
        }
        _ => SyncScope::Last30Days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chck_types::{AddAccountRequest, Page};

    #[test]
    fn repairs_legacy_headers_without_losing_body_index_or_cursor() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute_batch("INSERT INTO accounts (id,email,display_name,provider,mode,color,sync_scope,created_at) VALUES ('a','a@imyemail.test','a','custom','standard','#000','all',0);
            INSERT INTO folders (id,account_id,remote_name,path,role,sync_cursor) VALUES ('f','a','INBOX','INBOX','inbox','99');
            INSERT INTO messages (id,account_id,folder_id,uid,subject,from_json,snippet) VALUES ('m','a','f',1,'=?UTF-8?B?5rWL6K+V?=','[]','=?UTF-8?B?5rWL6K+V?=');
            INSERT INTO search_index (message_id,subject,from_addr,body) VALUES ('m','encoded','','preserved body');").unwrap();
        repair_encoded_headers(&conn).unwrap();
        repair_encoded_headers(&conn).unwrap();
        let (subject, snippet): (String, String) = conn
            .query_row("SELECT subject, snippet FROM messages WHERE id='m'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(subject, "测试");
        assert_eq!(snippet, "测试");
        let body: String = conn
            .query_row("SELECT body FROM search_index WHERE message_id='m'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(body, "preserved body");
        let cursor: String = conn
            .query_row("SELECT sync_cursor FROM folders WHERE id='f'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cursor, "99");
    }

    #[test]
    fn move_rekeys_body_attachment_and_search_atomically() {
        let store = Store::open_in_memory().unwrap();
        {
            let conn = store.conn.lock().unwrap();
            conn.execute_batch("INSERT INTO accounts (id,email,display_name,provider,mode,color,sync_scope,created_at) VALUES ('a','a@imyemail.test','a','custom','standard','#000','all',0);
                INSERT INTO folders (id,account_id,remote_name,path,role) VALUES ('inbox','a','INBOX','INBOX','inbox'),('archive','a','Archive','Archive','archive');
                INSERT INTO messages (id,account_id,folder_id,uid,subject,from_json) VALUES ('inbox:1','a','inbox',1,'kept','[]');
                INSERT INTO bodies(message_id,text) VALUES ('inbox:1','cached body');
                INSERT INTO attachments(id,message_id,name,mime,size,data) VALUES ('attachment','inbox:1','note.txt','text/plain',3,x'616263');
                INSERT INTO search_index(message_id,subject,body) VALUES ('inbox:1','kept','cached body');").unwrap();
        }
        let source = MessageId::from("inbox:1");
        let dest = store.get_folder(&FolderId::from("archive")).unwrap().unwrap();
        assert!(store.claim_move(&source, &dest.id).unwrap());
        assert!(!store.claim_move(&source, &dest.id).unwrap());
        store.move_message(&source, &dest, 42).unwrap();
        let moved = MessageId::from("archive:42");
        assert!(store.get_body(&source).unwrap().is_none());
        assert_eq!(store.get_body(&moved).unwrap().unwrap().text, "cached body");
        assert_eq!(store.list_attachments(&moved).unwrap().len(), 1);
        assert_eq!(
            store.attachment_data(&AttachmentId::from("attachment")).unwrap().unwrap(),
            b"abc"
        );
        assert_eq!(store.get_message_meta(&moved).unwrap().unwrap().2, 42);
        assert_eq!(
            store.search(&SearchQuery { text: "cached".into(), ..Default::default() }).unwrap()[0]
                .message_id,
            moved
        );
        assert!(
            store
                .move_message(&moved, &Folder { id: FolderId::from("missing"), ..dest.clone() }, 77)
                .is_err()
        );
        assert!(store.get_body(&moved).unwrap().is_some());
        store.set_folder_uidvalidity(&dest.id, 1).unwrap();
        store.set_folder_uidvalidity(&dest.id, 2).unwrap();
        assert!(store.get_message_meta(&moved).unwrap().is_none());
        assert!(store.get_body(&moved).unwrap().is_none());
        assert!(
            store
                .search(&SearchQuery { text: "cached".into(), ..Default::default() })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn migrates_to_current() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn account_roundtrip_and_cascade_delete() {
        let store = Store::open_in_memory().unwrap();
        let acc = Account {
            id: AccountId::from("acc1"),
            email: "a@qq.com".into(),
            display_name: "A".into(),
            provider_id: "qq".into(),
            mode: AccountMode::Standard,
            color: "#3D6BFE".into(),
            sync_scope: SyncScope::Last30Days,
            state: AccountState::Offline,
        };
        store.insert_account(&acc).unwrap();
        let mut renamed = acc.clone();
        renamed.display_name = "B".into();
        store.update_account(&renamed).unwrap();
        assert_eq!(store.get_account(&acc.id).unwrap().unwrap().display_name, "B");
        store.enqueue_op(&acc.id, "flag", "{}").unwrap();
        assert_eq!(store.list_accounts().unwrap().len(), 1);
        assert_eq!(store.pending_ops_count().unwrap(), 1);
        store.delete_account(&acc.id).unwrap();
        assert!(store.list_accounts().unwrap().is_empty());
        assert_eq!(store.pending_ops_count().unwrap(), 0);
        let _ = AddAccountRequest::new("a@qq.com");
    }

    #[test]
    fn folder_and_message_roundtrip() {
        let store = Store::open_in_memory().unwrap();
        let acc = Account {
            id: AccountId::from("acc1"),
            email: "dev@imyemail.test".into(),
            display_name: "Dev".into(),
            provider_id: "custom".into(),
            mode: AccountMode::Standard,
            color: "#3D6BFE".into(),
            sync_scope: SyncScope::Last30Days,
            state: AccountState::Offline,
        };
        store.insert_account(&acc).unwrap();
        let folder = Folder {
            id: FolderId::from("acc1:INBOX"),
            account_id: acc.id.clone(),
            remote_name: "INBOX".into(),
            path: "INBOX".into(),
            role: FolderRole::Inbox,
            unread_count: 0,
            total_count: 0,
        };
        store.upsert_folder(&folder).unwrap();
        store
            .upsert_message(
                &folder,
                &MessageRecord {
                    uid: 1,
                    subject: "Hello".into(),
                    from: vec![Address {
                        name: Some("Alice".into()),
                        email: "alice@imyemail.test".into(),
                    }],
                    to: vec![Address { name: None, email: "dev@imyemail.test".into() }],
                    date_unix: 100,
                    snippet: "Hello".into(),
                    flags: Flags::default(),
                    has_attachments: false,
                    message_id: Some("<id@imyemail.test>".into()),
                    in_reply_to: None,
                    thread_id: Some("<id@imyemail.test>".into()),
                    size: 12,
                },
            )
            .unwrap();
        let msgs = store.list_messages(&folder.id, Page { offset: 0, limit: 10 }).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].subject, "Hello");
        store
            .put_body(&MessageBody {
                id: msgs[0].id.clone(),
                html_sanitized: "<img src=\"https://images.example.test/mail.png\">".into(),
                text: "Hello".into(),
                remote_blocked: 1,
            })
            .unwrap();
        assert_eq!(store.get_body(&msgs[0].id).unwrap().unwrap().remote_blocked, 1);
        assert_eq!(store.message_count(&folder.id).unwrap(), (1, 1));
        let hits =
            store.search(&SearchQuery { text: "Hello".into(), ..SearchQuery::default() }).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].message_id.as_str(), msgs[0].id.as_str());
        let unified = store.unified_inbox(Page { offset: 0, limit: 10 }).unwrap();
        assert_eq!(unified.len(), 1);
        store
            .upsert_message(
                &folder,
                &MessageRecord {
                    uid: 2,
                    subject: "Re: Hello".into(),
                    from: vec![Address { name: None, email: "dev@imyemail.test".into() }],
                    to: vec![],
                    date_unix: 200,
                    snippet: "Re".into(),
                    flags: Flags::default(),
                    has_attachments: false,
                    message_id: Some("<re@imyemail.test>".into()),
                    in_reply_to: Some("<id@imyemail.test>".into()),
                    thread_id: store.find_thread_id(
                        &acc.id,
                        Some("<id@imyemail.test>"),
                        Some("<re@imyemail.test>"),
                    ),
                    size: 8,
                },
            )
            .unwrap();
        let thread = store.list_thread(&ThreadId::from("<id@imyemail.test>")).unwrap();
        assert_eq!(thread.len(), 2);
    }
}
