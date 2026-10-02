//! Loopback TLS servers: probes never send, submission supports implicit TLS and STARTTLS.
use chck_types::{AddAccountRequest, SendRequest};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn line(io: &mut (impl Read + Write)) -> String {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0];
        io.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8(bytes).unwrap()
}
fn reply(io: &mut (impl Read + Write), text: &str) {
    io.write_all(text.as_bytes()).unwrap();
    io.flush().unwrap();
}
fn server(starttls: bool, probe: bool) -> (u16, thread::JoinHandle<String>) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
    let config = Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.cert.der().clone()], key)
            .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut tcp, _) = listener.accept().unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        tcp.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        if starttls {
            reply(&mut tcp, "220 ready\r\n");
            assert!(line(&mut tcp).starts_with("EHLO "));
            reply(&mut tcp, "250-local\r\n250 STARTTLS\r\n");
            assert_eq!(line(&mut tcp), "STARTTLS\r\n");
            reply(&mut tcp, "220 upgrade\r\n");
        }
        let mut tls = StreamOwned::new(ServerConnection::new(config).unwrap(), tcp);
        if !starttls {
            reply(&mut tls, "220 ready\r\n");
        }
        assert!(line(&mut tls).starts_with("EHLO "));
        reply(&mut tls, "250-local\r\n250 AUTH LOGIN\r\n");
        assert_eq!(line(&mut tls), "AUTH LOGIN\r\n");
        reply(&mut tls, "334 user\r\n");
        assert_eq!(line(&mut tls), "ZGV2QGlteWVtYWlsLnRlc3Q=\r\n");
        reply(&mut tls, "334 password\r\n");
        assert_eq!(line(&mut tls), "c2VjcmV0\r\n");
        reply(&mut tls, "235 authenticated\r\n");
        let mut message = String::new();
        if !probe {
            assert_eq!(line(&mut tls), "MAIL FROM:<dev@imyemail.test>\r\n");
            reply(&mut tls, "250 sender\r\n");
            assert_eq!(line(&mut tls), "RCPT TO:<alice@imyemail.test>\r\n");
            reply(&mut tls, "250 recipient\r\n");
            assert_eq!(line(&mut tls), "DATA\r\n");
            reply(&mut tls, "354 content\r\n");
            loop {
                let next = line(&mut tls);
                if next == ".\r\n" {
                    break;
                }
                message.push_str(&next);
            }
            reply(&mut tls, "250 accepted\r\n");
        }
        assert_eq!(line(&mut tls), "QUIT\r\n");
        reply(&mut tls, "221 bye\r\n");
        message
    });
    (port, handle)
}
fn request(port: u16, starttls: bool) -> AddAccountRequest {
    let mut req = AddAccountRequest::new("dev@imyemail.test");
    req.smtp_host = Some("localhost".into());
    req.smtp_port = Some(port);
    req.smtp_starttls = starttls;
    req.password = Some("secret".into());
    req.accept_invalid_certs = true;
    req
}
#[test]
fn smtp_probe_authenticates_without_sending() {
    for starttls in [false, true] {
        let (port, handle) = server(starttls, true);
        chck_proto_smtp::test_connect(&request(port, starttls), None).unwrap();
        assert!(handle.join().unwrap().is_empty());
    }
}
#[test]
fn chinese_reply_roundtrips_through_tls_submission() {
    use mailparse::MailHeaderMap;
    for starttls in [false, true] {
        let (port, handle) = server(starttls, false);
        let send = SendRequest {
            to: vec!["alice@imyemail.test".into()],
            subject: "Re: 测试回复主题".repeat(12),
            body_text: "收到，谢谢！\n.开头的行\n新的一行".into(),
            ..SendRequest::default()
        };
        chck_proto_smtp::submit(&request(port, starttls), &send, None).unwrap();
        let raw = handle.join().unwrap();
        assert!(raw.is_ascii());
        let parsed = mailparse::parse_mail(raw.as_bytes()).unwrap();
        assert_eq!(parsed.headers.get_first_value("Subject").unwrap(), send.subject);
        assert_eq!(parsed.get_body().unwrap(), send.body_text);
    }
}

#[test]
fn markdown_alternatives_roundtrip_through_tls_submission() {
    for starttls in [false, true] {
        let (port, handle) = server(starttls, false);
        let send = SendRequest {
            to: vec!["alice@imyemail.test".into()],
            subject: "Markdown 中文邮件".into(),
            body_text: "# 标题\n\n**你好** 👋\n\n> 引用".into(),
            body_html: Some(
                "<h1>标题</h1><p><strong>你好</strong> 👋</p><blockquote>引用</blockquote>".into(),
            ),
            ..SendRequest::default()
        };
        chck_proto_smtp::submit(&request(port, starttls), &send, None).unwrap();
        let raw = handle.join().unwrap();
        let parsed = mailparse::parse_mail(raw.as_bytes()).unwrap();
        assert_eq!(parsed.ctype.mimetype, "multipart/alternative");
        assert_eq!(parsed.subparts.len(), 2);
        assert_eq!(parsed.subparts[0].get_body().unwrap(), send.body_text);
        assert_eq!(parsed.subparts[1].get_body().unwrap(), send.body_html.unwrap());
    }
}
