//! SMTP Submission。明文禁止；默认隐式 TLS（465），Outlook/iCloud 走 587 STARTTLS。

use chck_providers::{Provider, TlsMode};
use chck_types::{AddAccountRequest, EngineError, MailAuth, SendRequest};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Once};
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(20);

pub fn submit(
    req: &AddAccountRequest,
    send: &SendRequest,
    provider: Option<&Provider>,
) -> Result<(), EngineError> {
    if std::iter::once(req.email.as_str())
        .chain(send.to.iter().chain(&send.cc).chain(&send.bcc).map(String::as_str))
        .any(|address| address.contains(['\r', '\n', '<', '>']) || !address.contains('@'))
        || send.subject.contains(['\r', '\n'])
    {
        return Err(EngineError::Invalid("mail headers".into()));
    }
    let mut sess = authenticated_session(req, provider)?;
    let from = if req.email.contains('@') {
        req.email.clone()
    } else {
        return Err(EngineError::Invalid("sender email".into()));
    };
    sess.mail_from(&from)?;
    for rcpt in send.to.iter().chain(send.cc.iter()).chain(send.bcc.iter()) {
        sess.rcpt_to(rcpt)?;
    }
    sess.data(&build_rfc822(&from, send))?;
    let _ = sess.quit();
    Ok(())
}

/// Tests TLS and authentication without sending a message.
pub fn test_connect(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<(), EngineError> {
    let mut session = authenticated_session(req, provider)?;
    let _ = session.quit();
    Ok(())
}

fn authenticated_session(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<SmtpSession, EngineError> {
    let (host, port, starttls) = smtp_endpoint(req, provider)?;
    let addrs = (host.as_str(), port).to_socket_addrs().map_err(|_| EngineError::Network("dns"))?;
    let mut tcp = None;
    let mut last = EngineError::Network("refused");
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, IO_TIMEOUT) {
            Ok(s) => {
                tcp = Some(s);
                break;
            }
            Err(_) => last = EngineError::Network("refused"),
        }
    }
    let stream = tcp.ok_or(last)?;
    stream.set_read_timeout(Some(IO_TIMEOUT)).ok();
    stream.set_write_timeout(Some(IO_TIMEOUT)).ok();
    let mut sess = if starttls {
        let mut s = SmtpSession::plain(stream)?;
        s.expect(220)?;
        s.ehlo()?;
        s.write_line("STARTTLS")?;
        s.expect(220)?;
        s.upgrade_starttls(&host, req.accept_invalid_certs)?;
        s.ehlo()?;
        s
    } else {
        let mut s = SmtpSession::implicit_tls(stream, &host, req.accept_invalid_certs)?;
        s.expect(220)?;
        s.ehlo()?;
        s
    };
    if !sess.encrypted {
        return Err(EngineError::Tls);
    }
    let auth = req.mail_auth()?;
    match &auth {
        MailAuth::Xoauth2 { ir, .. } => {
            sess.auth_xoauth2(ir)?;
        }
        MailAuth::Login { user, password } => {
            sess.auth_login(user, password)?;
        }
    };
    Ok(sess)
}

pub fn smtp_endpoint(
    req: &AddAccountRequest,
    provider: Option<&Provider>,
) -> Result<(String, u16, bool), EngineError> {
    if let Some(host) = req.smtp_host.as_ref().filter(|h| !h.is_empty()) {
        return Ok((
            host.clone(),
            req.smtp_port.unwrap_or(if req.smtp_starttls { 587 } else { 465 }),
            req.smtp_starttls,
        ));
    }
    let host = req
        .smtp_host
        .clone()
        .or_else(|| provider.map(|p| p.smtp.host.clone()))
        .ok_or_else(|| EngineError::Invalid("smtp host".into()))?;
    let starttls =
        req.smtp_starttls || provider.is_some_and(|p| matches!(p.smtp.tls, TlsMode::Starttls));
    let port = req.smtp_port.or_else(|| provider.map(|p| p.smtp.port)).unwrap_or(if starttls {
        587
    } else {
        465
    });
    Ok((host, port, starttls))
}

enum SmtpIo {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for SmtpIo {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for SmtpIo {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

struct SmtpSession {
    io: Option<SmtpIo>,
    buf: Vec<u8>,
    encrypted: bool,
}

impl SmtpSession {
    fn plain(stream: TcpStream) -> Result<Self, EngineError> {
        Ok(Self { io: Some(SmtpIo::Plain(stream)), buf: Vec::new(), encrypted: false })
    }

    fn implicit_tls(
        stream: TcpStream,
        host: &str,
        accept_invalid: bool,
    ) -> Result<Self, EngineError> {
        let tls = wrap_tls(stream, host, accept_invalid)?;
        Ok(Self { io: Some(SmtpIo::Tls(Box::new(tls))), buf: Vec::new(), encrypted: true })
    }

    fn upgrade_starttls(&mut self, host: &str, accept_invalid: bool) -> Result<(), EngineError> {
        let Some(SmtpIo::Plain(stream)) = self.io.take() else {
            return Err(EngineError::Tls);
        };
        self.buf.clear();
        let tls = wrap_tls(stream, host, accept_invalid)?;
        self.io = Some(SmtpIo::Tls(Box::new(tls)));
        self.encrypted = true;
        Ok(())
    }

    fn io(&mut self) -> Result<&mut SmtpIo, EngineError> {
        self.io.as_mut().ok_or(EngineError::Tls)
    }

    fn read_reply(&mut self) -> Result<(u16, String), EngineError> {
        loop {
            if let Some((code, text, consumed)) = parse_reply(&self.buf) {
                self.buf.drain(..consumed);
                return Ok((code, text));
            }
            let mut tmp = [0u8; 1024];
            let n = self.io()?.read(&mut tmp).map_err(|_| EngineError::Network("io"))?;
            if n == 0 {
                return Err(EngineError::Network("closed"));
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }

    fn write_line(&mut self, line: &str) -> Result<(), EngineError> {
        self.io()?.write_all(line.as_bytes()).map_err(|_| EngineError::Network("io"))?;
        self.io()?.write_all(b"\r\n").map_err(|_| EngineError::Network("io"))?;
        self.io()?.flush().map_err(|_| EngineError::Network("io"))
    }

    fn expect(&mut self, want: u16) -> Result<String, EngineError> {
        let (code, text) = self.read_reply()?;
        if code == want { Ok(text) } else { Err(EngineError::Server("smtp")) }
    }

    fn ehlo(&mut self) -> Result<(), EngineError> {
        self.write_line("EHLO imy.email")?;
        let (code, _) = self.read_reply()?;
        if code == 250 { Ok(()) } else { Err(EngineError::Server("ehlo")) }
    }

    fn auth_login(&mut self, user: &str, pass: &str) -> Result<(), EngineError> {
        self.write_line("AUTH LOGIN")?;
        self.expect(334)?;
        self.write_line(&b64(user.as_bytes()))?;
        self.expect(334)?;
        self.write_line(&b64(pass.as_bytes()))?;
        match self.expect(235) {
            Ok(_) => Ok(()),
            Err(_) => Err(EngineError::AuthFailed),
        }
    }

    fn auth_xoauth2(&mut self, ir: &str) -> Result<(), EngineError> {
        self.write_line(&format!("AUTH XOAUTH2 {ir}"))?;
        let (code, _) = self.read_reply()?;
        if code == 235 {
            return Ok(());
        }
        if code == 334 {
            self.write_line("")?;
            let _ = self.read_reply();
        }
        Err(EngineError::AuthFailed)
    }

    fn mail_from(&mut self, from: &str) -> Result<(), EngineError> {
        self.write_line(&format!("MAIL FROM:<{from}>"))?;
        self.expect(250).map(|_| ())
    }

    fn rcpt_to(&mut self, to: &str) -> Result<(), EngineError> {
        self.write_line(&format!("RCPT TO:<{to}>"))?;
        self.expect(250).map(|_| ())
    }

    fn data(&mut self, body: &str) -> Result<(), EngineError> {
        self.write_line("DATA")?;
        self.expect(354)?;
        for line in body.lines() {
            if line.starts_with('.') {
                self.write_line(&format!(".{line}"))?;
            } else {
                self.write_line(line)?;
            }
        }
        self.write_line(".")?;
        self.expect(250).map(|_| ())
    }

    fn quit(&mut self) -> Result<(), EngineError> {
        self.write_line("QUIT")?;
        let _ = self.read_reply();
        Ok(())
    }
}

fn wrap_tls(
    mut stream: TcpStream,
    host: &str,
    accept_invalid: bool,
) -> Result<StreamOwned<ClientConnection, TcpStream>, EngineError> {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    let cfg = if accept_invalid {
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        ClientConfig::builder().with_root_certificates(roots).with_no_client_auth()
    };
    let name = ServerName::try_from(host.to_string()).map_err(|_| EngineError::Tls)?;
    let mut conn = ClientConnection::new(Arc::new(cfg), name).map_err(|_| EngineError::Tls)?;
    while conn.is_handshaking() {
        conn.complete_io(&mut stream).map_err(|_| EngineError::Tls)?;
    }
    Ok(StreamOwned::new(conn, stream))
}

fn parse_reply(buf: &[u8]) -> Option<(u16, String, usize)> {
    let text = std::str::from_utf8(buf).ok()?;
    let mut consumed = 0usize;
    for line in text.split_inclusive('\n') {
        if !line.ends_with('\n') || line.len() < 4 {
            return None;
        }
        let code = line.get(..3)?.parse().ok()?;
        consumed += line.len();
        if line.as_bytes().get(3) == Some(&b' ') {
            return Some((code, text[..consumed].to_string(), consumed));
        }
    }
    None
}

fn b64(input: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let remain = input.len() - i;
        let b0 = u32::from(input[i]);
        let b1 = if remain > 1 { u32::from(input[i + 1]) } else { 0 };
        let b2 = if remain > 2 { u32::from(input[i + 2]) } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if remain > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if remain > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

#[must_use]
pub fn rfc822(from: &str, send: &SendRequest) -> String {
    build_rfc822(from, send)
}

fn build_rfc822(from: &str, send: &SendRequest) -> String {
    let mut s = format!("From: {from}\r\nTo: {}\r\n", send.to.join(", "));
    if !send.cc.is_empty() {
        s.push_str(&format!("Cc: {}\r\n", send.cc.join(", ")));
    }
    let subject = if send.subject.is_ascii() {
        send.subject.clone()
    } else {
        let mut words = Vec::new();
        let mut chunk = String::new();
        for c in send.subject.chars() {
            if chunk.len() + c.len_utf8() > 42 {
                words.push(format!("=?UTF-8?B?{}?=", b64(chunk.as_bytes())));
                chunk.clear();
            }
            chunk.push(c);
        }
        if !chunk.is_empty() {
            words.push(format!("=?UTF-8?B?{}?=", b64(chunk.as_bytes())));
        }
        words.join("\r\n ")
    };
    s.push_str(&format!("Subject: {subject}\r\nMIME-Version: 1.0\r\n"));
    if let Some(html) = &send.body_html {
        // '-' never occurs in base64, so this boundary cannot collide with either encoded body.
        let boundary = "chck-markdown-alternative";
        s.push_str(&format!(
            "Content-Type: multipart/alternative; boundary=\"{boundary}\"\r\n\r\n\
             --{boundary}\r\n{}\r\n\
             --{boundary}\r\n{}\r\n\
             --{boundary}--\r\n",
            encoded_text_part("plain", &send.body_text),
            encoded_text_part("html", html),
        ));
    } else {
        s.push_str(&encoded_text_part("plain", &send.body_text));
    }
    s
}

fn encoded_text_part(subtype: &str, text: &str) -> String {
    let encoded = b64(text.as_bytes());
    let body = encoded
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n");
    format!(
        "Content-Type: text/{subtype}; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{body}"
    )
}

#[derive(Debug)]
struct NoVerify;

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_mail_preserves_both_unicode_alternatives() {
        let req = SendRequest {
            to: vec!["reader@example.test".into()],
            bcc: vec!["private@example.test".into()],
            subject: "中文格式邮件".into(),
            body_text: "## 标题\n\n**内容** 👋\n--chck-markdown-alternative".into(),
            body_html: Some("<h2>标题</h2><p><strong>内容</strong> 👋</p>".into()),
            ..SendRequest::default()
        };
        let wire = rfc822("sender@example.test", &req);
        let parsed = mailparse::parse_mail(wire.as_bytes()).unwrap();
        assert_eq!(parsed.ctype.mimetype, "multipart/alternative");
        assert_eq!(parsed.subparts.len(), 2);
        assert_eq!(parsed.subparts[0].ctype.mimetype, "text/plain");
        assert_eq!(parsed.subparts[0].get_body().unwrap(), req.body_text);
        assert_eq!(parsed.subparts[1].ctype.mimetype, "text/html");
        assert_eq!(parsed.subparts[1].get_body().unwrap(), req.body_html.unwrap());
        assert!(!wire.contains("private@example.test"));
    }

    #[test]
    fn plain_mail_remains_single_part() {
        let req = SendRequest { body_text: "纯文本 👋".into(), ..SendRequest::default() };
        let wire = rfc822("sender@example.test", &req);
        let parsed = mailparse::parse_mail(wire.as_bytes()).unwrap();
        assert_eq!(parsed.ctype.mimetype, "text/plain");
        assert!(parsed.subparts.is_empty());
        assert_eq!(parsed.get_body().unwrap(), req.body_text);
    }

    #[test]
    fn explicit_smtp_overrides_outlook_defaults() {
        let cat = chck_providers::ProviderCatalog::builtin();
        let p = cat.by_email("a@outlook.com").unwrap();
        let mut req = AddAccountRequest::new("a@outlook.com");
        req.smtp_host = Some("relay.example.test".into());
        req.smtp_port = Some(2465);
        assert_eq!(
            smtp_endpoint(&req, Some(p)).unwrap(),
            ("relay.example.test".into(), 2465, false)
        );
    }

    #[test]
    fn fragmented_smtp_reply_waits_for_newline() {
        assert!(parse_reply(b"250 OK").is_none());
        assert!(parse_reply(b"250-first\r\n250 final").is_none());
        assert_eq!(parse_reply(b"250-first\r\n250 final\r\n").unwrap().0, 250);
        assert!(parse_reply("你 好\r\n".as_bytes()).is_none());
    }

    #[test]
    fn outlook_endpoint_is_starttls_587() {
        let cat = chck_providers::ProviderCatalog::builtin();
        let p = cat.by_email("a@outlook.com").unwrap();
        let req = AddAccountRequest::new("a@outlook.com");
        let (host, port, starttls) = smtp_endpoint(&req, Some(p)).unwrap();
        assert_eq!(host, "smtp.office365.com");
        assert_eq!(port, 587);
        assert!(starttls);
    }

    #[test]
    fn qq_endpoint_is_implicit_465() {
        let cat = chck_providers::ProviderCatalog::builtin();
        let p = cat.by_email("a@qq.com").unwrap();
        let req = AddAccountRequest::new("a@qq.com");
        let (_, port, starttls) = smtp_endpoint(&req, Some(p)).unwrap();
        assert_eq!(port, 465);
        assert!(!starttls);
    }
}
