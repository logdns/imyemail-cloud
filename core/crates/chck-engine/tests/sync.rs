//! S2：添加账号 → LIST 文件夹 → 同步头 → 列表可读。

use chck_engine::CoreEngine;
use chck_types::{AddAccountRequest, FolderRole, MailEngine, Page};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn spawn_mail() -> u16 {
    spawn_mail_recording(true).0
}

fn spawn_mail_recording(uidplus: bool) -> (u16, Arc<Mutex<Vec<String>>>) {
    spawn_mail_behavior(uidplus, "OK [COPYUID 1 1 42] copied", false)
}

fn spawn_mail_behavior(
    uidplus: bool,
    copy_reply: &'static str,
    fail_delete: bool,
) -> (u16, Arc<Mutex<Vec<String>>>) {
    let commands = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&commands);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
            stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
            serve(stream, &recorded, uidplus, copy_reply, fail_delete);
        }
    });
    thread::sleep(Duration::from_millis(40));
    (port, commands)
}

fn serve(
    mut stream: TcpStream,
    recorded: &Mutex<Vec<String>>,
    uidplus: bool,
    copy_reply: &str,
    fail_delete: bool,
) {
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
    let _ = handle(&mut s, recorded, uidplus, copy_reply, fail_delete);
}

fn handle<S: Read + Write>(
    s: &mut S,
    recorded: &Mutex<Vec<String>>,
    uidplus: bool,
    copy_reply: &str,
    fail_delete: bool,
) -> std::io::Result<()> {
    let mut selected = false;
    let mut copied = false;
    writeln_imap(s, "* OK ready")?;
    let mut buf = Vec::new();
    while let Ok(line) = read_line(s, &mut buf) {
        let Some((tag, rest)) = line.split_once(' ') else { continue };
        if !rest.starts_with("LOGIN") {
            recorded.lock().unwrap().push(rest.into());
        }
        let cmd = rest.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        match cmd.as_str() {
            "CAPABILITY" => {
                writeln_imap(
                    s,
                    if uidplus {
                        "* CAPABILITY IMAP4rev1 AUTH=PLAIN IDLE UIDPLUS"
                    } else {
                        "* CAPABILITY IMAP4rev1 AUTH=PLAIN IDLE"
                    },
                )?;
                writeln_imap(s, &format!("{tag} OK"))?;
            }
            "LOGIN" => writeln_imap(s, &format!("{tag} OK LOGIN completed"))?,
            "LIST" => {
                writeln_imap(s, r#"* LIST (\HasNoChildren) "/" INBOX"#)?;
                writeln_imap(s, r#"* LIST (\Sent) "/" "Sent Messages""#)?;
                writeln_imap(s, r#"* LIST (\Archive) "/" Archive"#)?;
                writeln_imap(s, &format!("{tag} OK LIST completed"))?;
            }
            "SELECT" => {
                selected = true;
                writeln_imap(s, "* 1 EXISTS")?;
                writeln_imap(
                    s,
                    if copied && copy_reply.ends_with("source-reset") && rest.contains("INBOX") {
                        "* OK [UIDVALIDITY 2]"
                    } else {
                        "* OK [UIDVALIDITY 1]"
                    },
                )?;
                writeln_imap(s, "* OK [UIDNEXT 2]")?;
                writeln_imap(s, &format!("{tag} OK SELECT"))?;
            }
            "UID" => {
                if !selected {
                    writeln_imap(s, &format!("{tag} BAD no mailbox selected"))?;
                    continue;
                }
                if rest.starts_with("UID COPY ") {
                    copied = true;
                    writeln_imap(s, &format!("{tag} {copy_reply}"))?;
                    continue;
                }
                if rest.starts_with("UID STORE ") || rest.starts_with("UID EXPUNGE ") {
                    writeln_imap(
                        s,
                        &format!(
                            "{tag} {}",
                            if fail_delete { "NO deletion refused" } else { "OK done" }
                        ),
                    )?;
                    continue;
                }
                if rest.to_ascii_uppercase().contains("BODY.PEEK[]")
                    || rest.to_ascii_uppercase().contains("BODY[]")
                {
                    let raw = "From: Alice <alice@imyemail.test>\r\nSubject: Hello from testkit\r\nContent-Type: text/html\r\n\r\n<p>hi</p><script>alert(1)</script><img src=\"https://tracker.example/p.gif\">\r\n";
                    let n = raw.len();
                    writeln_imap(s, &format!("* 1 FETCH (UID 1 BODY[] {{{n}}}"))?;
                    s.write_all(raw.as_bytes())?;
                    writeln_imap(s, ")")?;
                    writeln_imap(s, &format!("{tag} OK FETCH"))?;
                } else {
                    let header = "From: Alice <alice@imyemail.test>\r\nSubject: Hello from testkit\r\nDate: Tue, 15 Sep 2026 10:00:00 +0800\r\n\r\n";
                    let n = header.len();
                    writeln_imap(
                        s,
                        &format!("* 1 FETCH (UID 1 FLAGS () RFC822.SIZE {n} BODY[HEADER] {{{n}}}"),
                    )?;
                    s.write_all(header.as_bytes())?;
                    writeln_imap(s, ")")?;
                    writeln_imap(s, &format!("{tag} OK FETCH"))?;
                }
            }
            "IDLE" => {
                writeln_imap(s, "+ idling")?;
                writeln_imap(s, "* 2 EXISTS")?;
                let _ = read_line(s, &mut buf);
                writeln_imap(s, &format!("{tag} OK IDLE terminated"))?;
            }
            "NOOP" => writeln_imap(s, &format!("{tag} OK"))?,
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

#[test]
fn add_account_discovers_folders_and_syncs_headers() {
    let port = spawn_mail();
    let engine = CoreEngine::open_in_memory().unwrap();
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.imap_host = Some("127.0.0.1".into());
    req.imap_port = Some(port);
    req.password = Some("devpass".into());
    req.accept_invalid_certs = true;
    let acc = engine.add_account(req).unwrap();
    let folders = engine.list_folders(&acc.id).unwrap();
    assert_eq!(folders.len(), 3);
    let inbox = folders.iter().find(|f| f.role == FolderRole::Inbox).unwrap();
    engine.sync_folder(&inbox.id).unwrap();
    let msgs = engine.list_messages(&inbox.id, Page::default()).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].subject, "Hello from testkit");
    assert_eq!(msgs[0].from[0].email, "alice@imyemail.test");
    assert!(!msgs[0].flags.seen);

    let body = engine.body(&msgs[0].id).unwrap();
    assert!(body.html_sanitized.contains("hi"));
    assert!(!body.html_sanitized.contains("script"));
    assert!(body.html_sanitized.contains("https://tracker.example/p.gif"));
    assert!(body.remote_blocked >= 1);
}

#[test]
fn tick_syncs_folders_without_idle_on_mobile() {
    let port = spawn_mail();
    let engine = CoreEngine::open_in_memory().unwrap();
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.imap_host = Some("127.0.0.1".into());
    req.imap_port = Some(port);
    req.password = Some("devpass".into());
    req.accept_invalid_certs = true;
    let acc = engine.add_account(req).unwrap();
    let ticks = engine.tick(Some(&acc.id), false, Some(0)).unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].state, chck_types::SyncState::Idle);
    assert!(ticks[0].supports_idle);
    let inbox = engine
        .list_folders(&acc.id)
        .unwrap()
        .into_iter()
        .find(|f| f.role == FolderRole::Inbox)
        .unwrap();
    let msgs = engine.list_messages(&inbox.id, Page::default()).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(engine.list_accounts().unwrap()[0].state, chck_types::AccountState::Online);
}

#[test]
fn tick_idle_wakes_on_exists() {
    let port = spawn_mail();
    let engine = CoreEngine::open_in_memory().unwrap();
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.imap_host = Some("127.0.0.1".into());
    req.imap_port = Some(port);
    req.password = Some("devpass".into());
    req.accept_invalid_certs = true;
    let acc = engine.add_account(req).unwrap();
    let ticks = engine.tick(Some(&acc.id), true, Some(2_000)).unwrap();
    assert_eq!(ticks[0].state, chck_types::SyncState::Done);
    assert!(ticks[0].supports_idle);
}

#[test]
fn archive_preserves_cached_body_and_uses_selected_source_and_mapped_uid() {
    let (port, commands) = spawn_mail_recording(true);
    let engine = CoreEngine::open_in_memory().unwrap();
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.imap_host = Some("127.0.0.1".into());
    req.imap_port = Some(port);
    req.password = Some("devpass".into());
    req.accept_invalid_certs = true;
    let account = engine.add_account(req).unwrap();
    let folders = engine.list_folders(&account.id).unwrap();
    let inbox = folders.iter().find(|f| f.role == FolderRole::Inbox).unwrap();
    let archive = folders.iter().find(|f| f.role == FolderRole::Archive).unwrap();
    engine.sync_folder(&inbox.id).unwrap();
    let message = engine.list_messages(&inbox.id, Page::default()).unwrap().remove(0);
    let before = engine.body(&message.id).unwrap();
    commands.lock().unwrap().clear();
    engine.move_message(&message.id, &archive.id).unwrap();
    assert!(engine.list_messages(&inbox.id, Page::default()).unwrap().is_empty());
    let moved = engine.list_messages(&archive.id, Page::default()).unwrap().remove(0);
    assert!(moved.id.as_str().ends_with(":42"));
    assert_eq!(engine.body(&moved.id).unwrap().html_sanitized, before.html_sanitized);
    let commands = commands.lock().unwrap();
    let select = commands.iter().position(|s| s.starts_with("SELECT ")).unwrap();
    let copy = commands.iter().position(|s| s.starts_with("UID COPY 1 ")).unwrap();
    assert!(select < copy);
    assert!(commands[select].contains("INBOX"));
    assert!(commands.iter().any(|s| s == "UID STORE 1 +FLAGS.SILENT (\\Deleted)"));
    assert!(commands.iter().any(|s| s == "UID EXPUNGE 1"));
    assert!(!commands.iter().any(|s| s == "EXPUNGE"));
}

#[test]
fn unsupported_archive_preserves_source_without_copy_or_delete() {
    let (port, commands) = spawn_mail_recording(false);
    let engine = CoreEngine::open_in_memory().unwrap();
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.imap_host = Some("127.0.0.1".into());
    req.imap_port = Some(port);
    req.password = Some("devpass".into());
    req.accept_invalid_certs = true;
    let account = engine.add_account(req).unwrap();
    let folders = engine.list_folders(&account.id).unwrap();
    let inbox = folders.iter().find(|f| f.role == FolderRole::Inbox).unwrap();
    let archive = folders.iter().find(|f| f.role == FolderRole::Archive).unwrap();
    engine.sync_folder(&inbox.id).unwrap();
    let message = engine.list_messages(&inbox.id, Page::default()).unwrap().remove(0);
    commands.lock().unwrap().clear();
    assert!(matches!(
        engine.move_message(&message.id, &archive.id),
        Err(chck_types::EngineError::Unsupported(_))
    ));
    assert_eq!(engine.list_messages(&inbox.id, Page::default()).unwrap().len(), 1);
    assert!(
        !commands.lock().unwrap().iter().any(|s| s.starts_with("UID COPY")
            || s.starts_with("UID STORE")
            || s.contains("EXPUNGE"))
    );
}

#[test]
fn ambiguous_archive_failure_keeps_source_and_prevents_repeated_copy() {
    for (reply, fail_delete) in [
        ("OK copied without mapping", false),
        ("OK [COPYUID 1 99 42] wrong source", false),
        ("OK [COPYUID 0 1 42] invalid validity", false),
        ("OK [COPYUID 1 1 0] invalid destination", false),
        ("OK [COPYUID 2 1 42] destination reset", false),
        ("OK [COPYUID 1 1 42] source-reset", false),
        ("OK [COPYUID 1 1 42] copied", true),
    ] {
        let (port, commands) = spawn_mail_behavior(true, reply, fail_delete);
        let engine = CoreEngine::open_in_memory().unwrap();
        let mut req = AddAccountRequest::new("dev@imyemail.test");
        req.imap_host = Some("127.0.0.1".into());
        req.imap_port = Some(port);
        req.password = Some("devpass".into());
        req.accept_invalid_certs = true;
        let account = engine.add_account(req).unwrap();
        let folders = engine.list_folders(&account.id).unwrap();
        let inbox = folders.iter().find(|f| f.role == FolderRole::Inbox).unwrap();
        let archive = folders.iter().find(|f| f.role == FolderRole::Archive).unwrap();
        engine.sync_folder(&inbox.id).unwrap();
        let message = engine.list_messages(&inbox.id, Page::default()).unwrap().remove(0);
        commands.lock().unwrap().clear();
        engine.move_message(&message.id, &inbox.id).unwrap();
        assert!(commands.lock().unwrap().is_empty());
        assert!(engine.move_message(&message.id, &archive.id).is_err(), "{reply}");
        assert!(engine.move_message(&message.id, &archive.id).is_err(), "retry {reply}");
        assert_eq!(engine.list_messages(&inbox.id, Page::default()).unwrap().len(), 1);
        let commands = commands.lock().unwrap();
        assert_eq!(commands.iter().filter(|s| s.starts_with("UID COPY ")).count(), 1, "{reply}");
        assert!(!commands.iter().any(|s| s.contains("EXPUNGE")), "{reply}");
        if !fail_delete {
            assert!(!commands.iter().any(|s| s.starts_with("UID STORE ")), "{reply}");
        }
    }
}

#[test]
fn delete_queue_never_uses_mailbox_wide_expunge() {
    for uidplus in [true, false] {
        let (port, commands) = spawn_mail_recording(uidplus);
        let engine = CoreEngine::open_in_memory().unwrap();
        let mut req = AddAccountRequest::new("dev@imyemail.test");
        req.imap_host = Some("127.0.0.1".into());
        req.imap_port = Some(port);
        req.password = Some("devpass".into());
        req.accept_invalid_certs = true;
        let account = engine.add_account(req).unwrap();
        let inbox = engine
            .list_folders(&account.id)
            .unwrap()
            .into_iter()
            .find(|f| f.role == FolderRole::Inbox)
            .unwrap();
        engine.sync_folder(&inbox.id).unwrap();
        let message = engine.list_messages(&inbox.id, Page::default()).unwrap().remove(0);
        commands.lock().unwrap().clear();
        engine.delete_message(&message.id).unwrap();
        let commands = commands.lock().unwrap();
        assert!(!commands.iter().any(|s| s == "EXPUNGE"));
        assert_eq!(commands.iter().any(|s| s == "UID EXPUNGE 1"), uidplus);
        assert_eq!(commands.iter().any(|s| s.starts_with("UID STORE ")), uidplus);
    }
}
