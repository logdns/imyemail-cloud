//! 本地 mock IMAP：隐式 TLS、强制 IMAP ID、明文禁止。

use chck_proto_imap::ImapClient;
use chck_providers::ProviderCatalog;
use chck_types::{AddAccountRequest, ProbeKind, ProbeStatus, xoauth2_ir};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn ensure_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn self_signed() -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
        .expect("cert");
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
    (cert_der, key)
}

fn server_config() -> Arc<ServerConfig> {
    ensure_crypto();
    let (cert, key) = self_signed();
    Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("server config"),
    )
}

#[derive(Clone, Copy)]
enum Mode {
    ImplicitTls { require_id: bool, user: &'static str, pass: &'static str },
    Xoauth2 { user: &'static str, token: &'static str },
    PlainNoStartTls,
}

fn spawn(mode: Mode) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
            stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
            match mode {
                Mode::ImplicitTls { require_id, user, pass } => {
                    serve_tls(stream, server_config(), AuthMode::Login { require_id, user, pass });
                }
                Mode::Xoauth2 { user, token } => {
                    serve_tls(stream, server_config(), AuthMode::Xoauth2 { user, token });
                }
                Mode::PlainNoStartTls => {
                    let _ = handle_imap(
                        stream,
                        AuthMode::Login {
                            require_id: false,
                            user: "dev@imyemail.test",
                            pass: "devpass",
                        },
                        false,
                    );
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(50));
    port
}

#[derive(Clone, Copy)]
enum AuthMode {
    Login { require_id: bool, user: &'static str, pass: &'static str },
    Xoauth2 { user: &'static str, token: &'static str },
}

fn serve_tls(mut stream: TcpStream, cfg: Arc<ServerConfig>, auth: AuthMode) {
    let mut conn = ServerConnection::new(cfg).unwrap();
    while conn.is_handshaking() {
        if conn.complete_io(&mut stream).is_err() {
            return;
        }
    }
    let tls = StreamOwned::new(conn, stream);
    let _ = handle_imap(tls, auth, true);
}

fn handle_imap<S: Read + Write>(mut s: S, auth: AuthMode, offer_id: bool) -> std::io::Result<()> {
    write_line(&mut s, "* OK IMAP4rev1 ready")?;
    let mut id_sent = false;
    let mut authed = false;
    let mut buf = Vec::new();
    while let Ok(line) = read_line(&mut s, &mut buf) {
        if line.is_empty() {
            continue;
        }
        let Some((tag, rest)) = line.split_once(' ') else { continue };
        let cmd = rest.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        match cmd.as_str() {
            "CAPABILITY" => {
                let caps = match (offer_id, auth) {
                    (true, AuthMode::Xoauth2 { .. }) => {
                        "* CAPABILITY IMAP4rev1 AUTH=PLAIN AUTH=XOAUTH2 ID IDLE"
                    }
                    (true, _) => "* CAPABILITY IMAP4rev1 AUTH=PLAIN ID IDLE",
                    (false, AuthMode::Xoauth2 { .. }) => {
                        "* CAPABILITY IMAP4rev1 AUTH=PLAIN AUTH=XOAUTH2 IDLE"
                    }
                    (false, _) => "* CAPABILITY IMAP4rev1 AUTH=PLAIN IDLE",
                };
                write_line(&mut s, caps)?;
                write_line(&mut s, &format!("{tag} OK CAPABILITY completed"))?;
            }
            "ID" => {
                id_sent = true;
                write_line(&mut s, "* ID NIL")?;
                write_line(&mut s, &format!("{tag} OK ID completed"))?;
            }
            "STARTTLS" => {
                write_line(&mut s, &format!("{tag} BAD no STARTTLS"))?;
            }
            "AUTHENTICATE" => {
                let AuthMode::Xoauth2 { user, token } = auth else {
                    write_line(&mut s, &format!("{tag} NO [AUTHENTICATIONFAILED]"))?;
                    continue;
                };
                let expected = format!("AUTHENTICATE XOAUTH2 {}", xoauth2_ir(user, token));
                if rest == expected {
                    authed = true;
                    write_line(&mut s, &format!("{tag} OK AUTHENTICATE completed"))?;
                } else {
                    write_line(&mut s, "+")?;
                    let _ = read_line(&mut s, &mut buf);
                    write_line(&mut s, &format!("{tag} NO [AUTHENTICATIONFAILED]"))?;
                }
            }
            "LOGIN" => {
                let AuthMode::Login { require_id, user, pass } = auth else {
                    write_line(&mut s, &format!("{tag} NO [AUTHENTICATIONFAILED]"))?;
                    continue;
                };
                if require_id && !id_sent {
                    write_line(&mut s, &format!("{tag} NO Unsafe Login"))?;
                    continue;
                }
                let expected = format!("LOGIN \"{user}\" \"{pass}\"");
                if rest == expected {
                    authed = true;
                    write_line(&mut s, &format!("{tag} OK LOGIN completed"))?;
                } else {
                    write_line(&mut s, &format!("{tag} NO [AUTHENTICATIONFAILED]"))?;
                }
            }
            "LIST" => {
                if !authed {
                    write_line(&mut s, &format!("{tag} NO not authenticated"))?;
                    continue;
                }
                write_line(&mut s, r#"* LIST (\HasNoChildren) "/" INBOX"#)?;
                write_line(&mut s, r#"* LIST (\Sent) "/" "Sent Messages""#)?;
                write_line(&mut s, &format!("{tag} OK LIST completed"))?;
            }
            "SELECT" => {
                write_line(&mut s, "* 1 EXISTS")?;
                write_line(&mut s, "* OK [UIDVALIDITY 1] UIDs valid")?;
                write_line(&mut s, "* OK [UIDNEXT 2] Predicted next UID")?;
                write_line(&mut s, &format!("{tag} OK [READ-WRITE] SELECT completed"))?;
            }
            "UID" => {
                let header = "From: Alice <alice@imyemail.test>\r\nTo: Dev <dev@imyemail.test>\r\nSubject: Hello from testkit\r\nDate: Tue, 15 Sep 2026 10:00:00 +0800\r\nMessage-ID: <simple-001@imyemail.test>\r\n\r\n";
                let n = header.len();
                write_line(
                    &mut s,
                    &format!("* 1 FETCH (UID 1 FLAGS () RFC822.SIZE {n} BODY[HEADER] {{{n}}}"),
                )?;
                s.write_all(header.as_bytes())?;
                write_line(&mut s, ")")?;
                write_line(&mut s, &format!("{tag} OK FETCH completed"))?;
            }
            "APPEND" => {
                let n = rest
                    .rfind('{')
                    .and_then(|i| rest[i + 1..].trim_end_matches('}').parse::<usize>().ok())
                    .unwrap_or(0);
                write_line(&mut s, "+")?;
                let mut got = 0usize;
                while got < n {
                    if !buf.is_empty() {
                        let take = (n - got).min(buf.len());
                        buf.drain(..take);
                        got += take;
                        continue;
                    }
                    let mut tmp = [0u8; 1024];
                    let r = s.read(&mut tmp)?;
                    if r == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..r]);
                }
                let _ = read_line(&mut s, &mut buf);
                write_line(&mut s, &format!("{tag} OK APPEND"))?;
            }
            "IDLE" => {
                write_line(&mut s, "+ idling")?;
                write_line(&mut s, "* 2 EXISTS")?;
                let _ = read_line(&mut s, &mut buf);
                write_line(&mut s, &format!("{tag} OK IDLE terminated"))?;
            }
            "NOOP" => write_line(&mut s, &format!("{tag} OK NOOP"))?,
            "LOGOUT" => {
                write_line(&mut s, "* BYE")?;
                write_line(&mut s, &format!("{tag} OK LOGOUT completed"))?;
                break;
            }
            _ => write_line(&mut s, &format!("{tag} BAD unknown"))?,
        }
    }
    Ok(())
}

fn write_line<W: Write>(w: &mut W, line: &str) -> std::io::Result<()> {
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
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof"));
        }
        buf.extend_from_slice(&tmp[..n]);
    }
}

fn req(email: &str, host: &str, port: u16, password: &str) -> AddAccountRequest {
    let mut req = AddAccountRequest::new(email);
    req.imap_host = Some(host.into());
    req.imap_port = Some(port);
    req.password = Some(password.into());
    req.accept_invalid_certs = true;
    req
}

fn req_oauth(email: &str, host: &str, port: u16, token: &str) -> AddAccountRequest {
    let mut req = AddAccountRequest::new(email);
    req.imap_host = Some(host.into());
    req.imap_port = Some(port);
    req.access_token = Some(token.into());
    req.accept_invalid_certs = true;
    req
}

#[test]
fn implicit_tls_login_and_list() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let probe =
        ImapClient::test_connect(&req("dev@imyemail.test", "127.0.0.1", port, "devpass"), None)
            .unwrap();
    assert!(probe.success(), "{probe:?}");
    assert_eq!(probe.step(ProbeKind::Tls).unwrap().status, ProbeStatus::Ok);
    assert_eq!(probe.step(ProbeKind::Auth).unwrap().status, ProbeStatus::Ok);
    assert_eq!(probe.step(ProbeKind::Folders).unwrap().status, ProbeStatus::Ok);
}

#[test]
fn qq_quirk_sends_id_before_login() {
    let port = spawn(Mode::ImplicitTls { require_id: true, user: "a@qq.com", pass: "authcode" });
    let cat = ProviderCatalog::builtin();
    let qq = cat.by_email("a@qq.com").unwrap();
    let probe = ImapClient::test_connect(&req("a@qq.com", "127.0.0.1", port, "authcode"), Some(qq))
        .unwrap();
    assert!(probe.success(), "{probe:?}");
}

#[test]
fn wrong_password_fails_auth_only() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let probe =
        ImapClient::test_connect(&req("dev@imyemail.test", "127.0.0.1", port, "nope"), None)
            .unwrap();
    assert!(!probe.success());
    assert_eq!(probe.step(ProbeKind::Tls).unwrap().status, ProbeStatus::Ok);
    assert_eq!(probe.step(ProbeKind::Auth).unwrap().status, ProbeStatus::Failed);
    assert_eq!(probe.step(ProbeKind::Folders).unwrap().status, ProbeStatus::Skipped);
}

#[test]
fn plaintext_without_starttls_is_forbidden() {
    let port = spawn(Mode::PlainNoStartTls);
    let mut r = req("dev@imyemail.test", "127.0.0.1", port, "devpass");
    r.imap_starttls = true;
    r.accept_invalid_certs = false;
    let probe = ImapClient::test_connect(&r, None).unwrap();
    assert!(!probe.success(), "{probe:?}");
    let tls = probe.step(ProbeKind::Tls).unwrap();
    assert_eq!(tls.status, ProbeStatus::Failed);
    assert!(tls.detail.contains("plaintext") || tls.detail.contains("STARTTLS"));
}

#[test]
fn xoauth2_authenticate_lists_inbox() {
    let port = spawn(Mode::Xoauth2 { user: "a@gmail.com", token: "ya29.token" });
    let probe =
        ImapClient::test_connect(&req_oauth("a@gmail.com", "127.0.0.1", port, "ya29.token"), None)
            .unwrap();
    assert!(probe.success(), "{probe:?}");
    assert_eq!(probe.step(ProbeKind::Auth).unwrap().status, ProbeStatus::Ok);
    let mut session =
        ImapClient::login(&req_oauth("a@gmail.com", "127.0.0.1", port, "ya29.token"), None)
            .unwrap();
    let folders = session.list_folders().unwrap();
    assert!(folders.iter().any(|f| f.role == chck_types::FolderRole::Inbox));
    session.logout();
}

#[test]
fn login_lists_inbox_and_fetches_headers() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let mut session =
        ImapClient::login(&req("dev@imyemail.test", "127.0.0.1", port, "devpass"), None).unwrap();
    let folders = session.list_folders().unwrap();
    assert!(folders.iter().any(|f| f.role == chck_types::FolderRole::Inbox));
    assert!(folders.iter().any(|f| f.role == chck_types::FolderRole::Sent));
    session.select("INBOX").unwrap();
    let items = session.fetch_headers_since(1, 50).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].headers.subject, "Hello from testkit");
    assert_eq!(items[0].headers.from[0].email, "alice@imyemail.test");
    session.logout();
}

#[test]
fn idle_wakes_on_exists() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let mut session =
        ImapClient::login(&req("dev@imyemail.test", "127.0.0.1", port, "devpass"), None).unwrap();
    session.select("INBOX").unwrap();
    assert!(session.supports_idle().unwrap());
    let wake = session.idle_wait(std::time::Duration::from_secs(2)).unwrap();
    assert_eq!(wake, chck_proto_imap::IdleWake::Exists(2));
    session.logout();
}

#[test]
fn append_rfc822_to_sent() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let mut session =
        ImapClient::login(&req("dev@imyemail.test", "127.0.0.1", port, "devpass"), None).unwrap();
    session.append_rfc822("Sent Messages", "From: a@b.com\r\nSubject: x\r\n\r\nhello").unwrap();
    session.logout();
}

#[test]
fn untrusted_cert_rejected_by_default() {
    let port =
        spawn(Mode::ImplicitTls { require_id: false, user: "dev@imyemail.test", pass: "devpass" });
    let mut r = req("dev@imyemail.test", "127.0.0.1", port, "devpass");
    r.accept_invalid_certs = false;
    let probe = ImapClient::test_connect(&r, None).unwrap();
    assert!(!probe.success());
    assert_eq!(probe.step(ProbeKind::Tls).unwrap().status, ProbeStatus::Failed);
}
