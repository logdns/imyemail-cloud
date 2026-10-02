use chck_types::EngineError;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<(), EngineError> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(map_err)?;
    if version < 1 {
        conn.execute_batch(V1).map_err(map_err)?;
        conn.pragma_update(None, "user_version", 1).map_err(map_err)?;
    }
    if version < 2 {
        conn.execute_batch(V2).map_err(map_err)?;
        conn.pragma_update(None, "user_version", 2).map_err(map_err)?;
    }
    if version < 3 {
        conn.execute_batch(V3).map_err(map_err)?;
        conn.pragma_update(None, "user_version", 3).map_err(map_err)?;
    }
    if version < 4 {
        conn.execute_batch(V4).map_err(map_err)?;
        conn.pragma_update(None, "user_version", 4).map_err(map_err)?;
    }
    Ok(())
}

fn map_err(err: rusqlite::Error) -> EngineError {
    EngineError::Storage(err.to_string())
}

const V1: &str = r"
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
";

const V2: &str = r"
ALTER TABLE messages ADD COLUMN snooze_until INTEGER;
CREATE TABLE IF NOT EXISTS signatures (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    body TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0
);
";

const V3: &str = r"
ALTER TABLE attachments ADD COLUMN data BLOB;
";

// Before v4 the sanitizer permanently removed remote image URLs and the store
// did not persist the blocked count. Invalidate only cached HTML containing
// images so those messages are fetched and sanitized with the new policy when
// next opened; ordinary text/HTML remains available offline.
const V4: &str = r"
ALTER TABLE bodies ADD COLUMN remote_blocked INTEGER NOT NULL DEFAULT 0;
UPDATE messages SET body_state = 'headers'
WHERE id IN (SELECT message_id FROM bodies WHERE html_sanitized LIKE '%<img%');
DELETE FROM bodies WHERE html_sanitized LIKE '%<img%';
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_invalidates_only_legacy_image_bodies() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(V1).unwrap();
        conn.execute_batch(V2).unwrap();
        conn.execute_batch(V3).unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();
        conn.execute_batch(
            "INSERT INTO accounts (id,email,display_name,provider,mode,color,sync_scope,created_at)
             VALUES ('a','a@example.test','A','custom','standard','#000','all',0);
             INSERT INTO folders (id,account_id,remote_name,path,role)
             VALUES ('f','a','INBOX','INBOX','inbox');
             INSERT INTO messages (id,account_id,folder_id,uid,body_state)
             VALUES ('plain','a','f',1,'full'),('image','a','f',2,'full');
             INSERT INTO bodies (message_id,html_sanitized,text)
             VALUES ('plain','<p>kept</p>','kept'),('image','<p>old</p><img>','old');",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let plain_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bodies WHERE message_id='plain'", [], |row| row.get(0))
            .unwrap();
        let image_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bodies WHERE message_id='image'", [], |row| row.get(0))
            .unwrap();
        let image_state: String = conn
            .query_row("SELECT body_state FROM messages WHERE id='image'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(plain_count, 1);
        assert_eq!(image_count, 0);
        assert_eq!(image_state, "headers");
        assert_eq!(
            conn.query_row(
                "SELECT remote_blocked FROM bodies WHERE message_id='plain'",
                [],
                |row| { row.get::<_, u32>(0) }
            )
            .unwrap(),
            0
        );
    }
}
