use chck_engine::CoreEngine;
use chck_ffi::FfiEngine;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Fixture {
    engine: Arc<FfiEngine>,
    observer: Connection,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("chck-cached-body-{}-{unique}-{sequence}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("mail.db");
        let engine = Arc::new(FfiEngine::open_with_file_secrets(db.to_str().unwrap()).unwrap());
        let observer = Connection::open(&db).unwrap();
        observer
            .execute_batch(
                "PRAGMA foreign_keys=ON;
            INSERT INTO accounts(id,email,display_name,provider,mode,color,sync_scope,created_at)
                VALUES ('a','test@example.test','Test','custom','standard','#000','all',0);
            INSERT INTO folders(id,account_id,remote_name,path,role)
                VALUES ('a:INBOX','a','INBOX','INBOX','inbox');
            INSERT INTO messages(id,account_id,folder_id,uid,flags,subject,from_json)
                VALUES ('a:INBOX:1','a','a:INBOX',1,0,'Cached','[]'),
                       ('a:INBOX:2','a','a:INBOX',2,0,'Uncached','[]');
            INSERT INTO bodies(message_id,html_sanitized,text,fetched_at)
                VALUES ('a:INBOX:1','<p>Already sanitized</p>','Already sanitized',123);",
            )
            .unwrap();
        Self { engine, observer, dir }
    }

    fn data_version(&self) -> i64 {
        self.observer.query_row("PRAGMA data_version", [], |r| r.get(0)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn cached_body_is_read_only_and_uses_existing_body_contract() {
    let fixture = Fixture::new();
    let before = fixture.data_version();
    let args = r#"{"message_id":"a:INBOX:1"}"#;
    let cached = fixture.engine.call("cached_body", args).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&cached).unwrap(),
        json!({
            "id": "a:INBOX:1", "html_sanitized": "<p>Already sanitized</p>",
            "text": "Already sanitized", "remote_blocked": 0,
        })
    );
    // No account transport configuration exists: body must use its cache first.
    assert_eq!(fixture.engine.call("body", args).unwrap(), cached);
    for id in ["a:INBOX:2", "missing"] {
        assert_eq!(
            fixture.engine.call("cached_body", &json!({"message_id": id}).to_string()).unwrap(),
            "null"
        );
    }
    assert!(fixture.engine.call("cached_body", "{}").unwrap_err().contains("message_id"));
    assert!(fixture.engine.call("cached_body", r#"{"message_id":123}"#).is_err());
    let flags: i64 =
        fixture.observer.query_row("SELECT SUM(flags) FROM messages", [], |r| r.get(0)).unwrap();
    assert_eq!(flags, 0);
    assert_eq!(fixture.data_version(), before, "cache reads must not write SQLite");
}

#[test]
fn cached_body_can_complete_while_another_call_waits_on_network() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CoreEngine>();
    assert_send_sync::<FfiEngine>();

    let fixture = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let settings = json!({"email":"test@example.test", "imap_host":"127.0.0.1",
        "imap_port": listener.local_addr().unwrap().port(), "accept_invalid_certs":true});
    fixture
        .observer
        .execute("INSERT INTO settings(key,value) VALUES ('imap:a',?1)", [settings.to_string()])
        .unwrap();
    let (accepted_tx, accepted_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        accepted_tx.send(()).unwrap();
        // Hold the TLS handshake until the cache-only request has finished.
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
        drop(stream);
    });
    let engine = Arc::clone(&fixture.engine);
    // Keep testing that a cache lookup is not blocked by an unrelated network diagnostic.
    let args = json!({"json":settings.to_string()}).to_string();
    let network = std::thread::spawn(move || engine.call("test_account", &args));
    accepted_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let engine = Arc::clone(&fixture.engine);
    let (result_tx, result_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        result_tx.send(engine.call("cached_body", r#"{"message_id":"a:INBOX:1"}"#)).unwrap();
    });
    let cached = result_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    server.join().unwrap();
    let _diagnostic = network.join().unwrap();
    reader.join().unwrap();
    let value: Value =
        serde_json::from_str(&cached.expect("cache lookup waited for network").unwrap()).unwrap();
    assert_eq!(value["text"], "Already sanitized");
}

#[test]
fn clear_failed_queue_preserves_drafts_active_work_and_move_claims() {
    let fixture = Fixture::new();
    for state in ["failed", "queued", "sending", "done", "draft", "sent", "scheduled", "cancelled"]
    {
        fixture.observer.execute(
            "INSERT INTO outbox(id,account_id,draft_json,state,retry_count,scheduled_at,error) VALUES (?1,'a',?2,?1,2,123,'original')",
            [state, r#"{"body_text":"Preserve composed content","to":["recipient@example.test"]}"#],
        ).unwrap();
        fixture.observer.execute(
            "INSERT INTO ops_queue(account_id,type,payload_json,created_at,state) VALUES ('a','flag','{}',0,?1)", [state],
        ).unwrap();
    }
    fixture
        .observer
        .execute("INSERT INTO settings(key,value) VALUES ('move:protected','Archive')", [])
        .unwrap();
    fixture
        .observer
        .execute("INSERT INTO settings(key,value) VALUES ('imap:a','credential-reference')", [])
        .unwrap();
    type OutboxSnapshot = (String, String, String, i64, Option<i64>, Option<String>);
    let untouched = || -> Vec<OutboxSnapshot> {
        fixture.observer.prepare("SELECT id,draft_json,state,retry_count,scheduled_at,error FROM outbox WHERE id != 'failed' ORDER BY id").unwrap()
            .query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).unwrap()
            .collect::<rusqlite::Result<_>>().unwrap()
    };
    let before = untouched();
    let output: Value =
        serde_json::from_str(&fixture.engine.call("clear_failed_queue", "{}").unwrap()).unwrap();
    assert_eq!(output, json!({"ops_cleared":1,"outbox_cleared":1,"total":2}));
    assert_eq!(untouched(), before);
    let recovered: (String, String, i64, Option<i64>, Option<String>) = fixture
        .observer
        .query_row(
            "SELECT draft_json,state,retry_count,scheduled_at,error FROM outbox WHERE id='failed'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert!(recovered.0.contains("Preserve composed content"));
    assert_eq!(
        (recovered.1, recovered.2, recovered.3, recovered.4),
        ("draft".into(), 0, None, None)
    );
    let remaining: i64 =
        fixture.observer.query_row("SELECT COUNT(*) FROM ops_queue", [], |row| row.get(0)).unwrap();
    assert_eq!(remaining, 7);
    for (key, value) in [("move:protected", "Archive"), ("imap:a", "credential-reference")] {
        let actual: String = fixture
            .observer
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, value);
    }
    let again: Value =
        serde_json::from_str(&fixture.engine.call("clear_failed_queue", "{}").unwrap()).unwrap();
    assert_eq!(again, json!({"ops_cleared":0,"outbox_cleared":0,"total":0}));
}
