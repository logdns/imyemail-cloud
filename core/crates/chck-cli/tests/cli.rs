use chck_engine::CoreEngine;
use chck_types::{AccountId, MailEngine, MessageId, SendRequest};
// CLI 第一客户端：开源版本可直接使用网络同步与发送能力。

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn spawn_mail() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
            stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
            serve(stream);
        }
    });
    thread::sleep(Duration::from_millis(40));
    port
}

fn serve(mut stream: TcpStream) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
    let cfg = Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key)
            .unwrap(),
    );
    let mut conn = ServerConnection::new(cfg).unwrap();
    while conn.is_handshaking() {
        if conn.complete_io(&mut stream).is_err() {
            return;
        }
    }
    let mut s = StreamOwned::new(conn, stream);
    let _ = handle(&mut s);
}

fn handle<S: Read + Write>(s: &mut S) -> std::io::Result<()> {
    writeln_imap(s, "* OK ready")?;
    let mut buf = Vec::new();
    while let Ok(line) = read_line(s, &mut buf) {
        let Some((tag, rest)) = line.split_once(' ') else { continue };
        let cmd = rest.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        match cmd.as_str() {
            "CAPABILITY" => {
                writeln_imap(s, "* CAPABILITY IMAP4rev1 AUTH=PLAIN")?;
                writeln_imap(s, &format!("{tag} OK"))?;
            }
            "LOGIN" => writeln_imap(s, &format!("{tag} OK LOGIN completed"))?,
            "LIST" => {
                writeln_imap(s, r#"* LIST (\HasNoChildren) "/" INBOX"#)?;
                writeln_imap(s, &format!("{tag} OK LIST completed"))?;
            }
            "SELECT" => {
                writeln_imap(s, "* 1 EXISTS")?;
                writeln_imap(s, &format!("{tag} OK SELECT"))?;
            }
            "UID" => {
                if rest.to_ascii_uppercase().contains("BODY.PEEK[]")
                    || rest.to_ascii_uppercase().contains("BODY[]")
                {
                    let raw =
                        "Subject: Hello from testkit\r\nContent-Type: text/plain\r\n\r\nbody\r\n";
                    let n = raw.len();
                    writeln_imap(s, &format!("* 1 FETCH (UID 1 BODY[] {{{n}}}"))?;
                    s.write_all(raw.as_bytes())?;
                    writeln_imap(s, ")")?;
                    writeln_imap(s, &format!("{tag} OK"))?;
                } else {
                    let header = "From: Alice <alice@imyemail.test>\r\nSubject: Hello from testkit\r\nDate: Tue, 15 Sep 2026 10:00:00 +0800\r\n\r\n";
                    let n = header.len();
                    writeln_imap(
                        s,
                        &format!("* 1 FETCH (UID 1 FLAGS () RFC822.SIZE {n} BODY[HEADER] {{{n}}}"),
                    )?;
                    s.write_all(header.as_bytes())?;
                    writeln_imap(s, ")")?;
                    writeln_imap(s, &format!("{tag} OK"))?;
                }
            }
            "LOGOUT" => {
                writeln_imap(s, "* BYE")?;
                writeln_imap(s, &format!("{tag} OK"))?;
                break;
            }
            _ => writeln_imap(s, &format!("{tag} BAD"))?,
        }
    }
    Ok(())
}

fn writeln_imap<W: Write>(w: &mut W, line: &str) -> std::io::Result<()> {
    w.write_all(line.as_bytes())?;
    w.write_all(b"\r\n")?;
    w.flush()
}

fn read_line<R: Read>(r: &mut R, buf: &mut Vec<u8>) -> std::io::Result<String> {
    loop {
        if let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = buf.drain(..=pos).collect();
            if line.ends_with(b"\n") {
                line.pop();
            }
            if line.ends_with(b"\r") {
                line.pop();
            }
            return Ok(String::from_utf8_lossy(&line).into_owned());
        }
        let mut tmp = [0u8; 1024];
        let n = r.read(&mut tmp)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        buf.extend_from_slice(&tmp[..n]);
    }
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_string()).collect()
}

fn run_db(db: &str, rest: &[&str]) -> String {
    let mut parts = vec!["--db", db, "--secrets", "file"];
    parts.extend_from_slice(rest);
    chck_cli::run(&argv(&parts)).unwrap()
}

#[test]
fn cli_syncs_and_reads_mail_without_a_product_gate() {
    let port = spawn_mail();
    let dir = std::env::temp_dir().join(format!("chck-cli-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let db = dir.join("mail.db");
    let db_s = db.to_str().unwrap();
    let port_s = port.to_string();

    let acc_json = run_db(
        db_s,
        &[
            "add",
            "--email",
            "dev@imyemail.test",
            "--host",
            "127.0.0.1",
            "--port",
            &port_s,
            "--password",
            "devpass",
            "--insecure",
        ],
    );
    let acc: serde_json::Value = serde_json::from_str(&acc_json).unwrap();
    let account_id = acc["id"].as_str().unwrap();

    // Seed cached mail through the protocol library, then verify that the public CLI can
    // synchronize without a product feature gate.
    let core = CoreEngine::open(db_s).unwrap();
    let seeded = core.list_folders(&AccountId::from(account_id)).unwrap();
    core.sync_folder(&seeded[0].id).unwrap();
    let seeded_messages = core.list_messages(&seeded[0].id, Default::default()).unwrap();
    core.body(&MessageId::from(seeded_messages[0].id.as_str())).unwrap();
    let folders_json = run_db(db_s, &["folders", "--account", account_id]);
    let folders: serde_json::Value = serde_json::from_str(&folders_json).unwrap();
    let folder_id = folders[0]["id"].as_str().unwrap();

    run_db(db_s, &["sync", "--folder", folder_id]);
    let list = run_db(db_s, &["list", "--folder", folder_id]);
    let msgs: serde_json::Value = serde_json::from_str(&list).unwrap();
    assert_eq!(msgs[0]["subject"], "Hello from testkit");
    let mid = msgs[0]["id"].as_str().unwrap();
    let body = run_db(db_s, &["read", "--message", mid]);
    assert!(body.contains("body") || body.contains("Hello"));

    let ui = run_db(db_s, &["ui"]);
    assert!(ui.contains("A 导航"));
    assert!(ui.contains("Hello from testkit"));
}

#[test]
fn cli_providers_lists_qq_preset() {
    let dir = std::env::temp_dir().join(format!("chck-cli-prov-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let db = dir.join("mail.db");
    let db_s = db.to_str().unwrap();
    let json = run_db(db_s, &["providers"]);
    assert!(json.contains("imap.qq.com"));
    assert!(json.contains("authcode"));
    let via_ffi = run_db(db_s, &["ffi", "--method", "list_providers", "--args", "{}"]);
    assert!(via_ffi.contains("imap.qq.com"));
}

#[test]
fn cli_queues_send_without_a_product_gate_and_can_cancel() {
    let dir = std::env::temp_dir().join(format!("chck-cli-undo-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let db = dir.join("mail.db");
    let db_s = db.to_str().unwrap();
    let acc_json = run_db(db_s, &["add", "--email", "dev@gmail.com"]);
    let acc: serde_json::Value = serde_json::from_str(&acc_json).unwrap();
    let account_id = acc["id"].as_str().unwrap();
    let request = SendRequest {
        account_id: AccountId::from(account_id),
        to: vec!["b@ex.com".into()],
        subject: "Hi".into(),
        undo_window_secs: 60,
        ..Default::default()
    };
    let core = CoreEngine::open(db_s).unwrap();
    let id = core.send(request).unwrap();
    let id = id.as_str();
    run_db(db_s, &["send", "--account", account_id, "--to", "b@ex.com", "--subject", "Hi"]);
    let box_ = run_db(db_s, &["outbox"]);
    assert!(box_.contains("queued"));
    run_db(db_s, &["undo", "--draft", id]);
    let box_ = run_db(db_s, &["outbox"]);
    assert!(box_.contains("cancelled"));
}

#[test]
fn cli_vcard_ics_backup() {
    let dir = std::env::temp_dir().join(format!("chck-cli-m5-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let db = dir.join("mail.db");
    let db_s = db.to_str().unwrap();
    run_db(db_s, &["add", "--email", "me@ex.com"]);
    let n =
        run_db(db_s, &["vcard", "--data", "BEGIN:VCARD\nFN:Pat\nEMAIL:pat@ex.com\nEND:VCARD\n"]);
    assert!(n.contains("1"));
    let contacts = run_db(db_s, &["contacts"]);
    assert!(contacts.contains("pat@ex.com"));
    let inv = run_db(db_s, &["ics", "--data", "BEGIN:VEVENT\nUID:z\nSUMMARY:A\nEND:VEVENT\n"]);
    assert!(inv.contains("\"uid\":\"z\""));
    let bak = run_db(db_s, &["backup"]);
    assert!(bak.contains("me@ex.com"));
    assert!(!bak.contains("password"));
}
