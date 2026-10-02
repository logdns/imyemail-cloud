use crate::tls::{client_config, ensure_crypto, hex_sha256};
use chck_types::EngineError;
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_LINE: usize = 64 * 1024;

pub enum Transport {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for Transport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Transport {
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

pub struct ImapSession {
    transport: Option<Transport>,
    read_buf: Vec<u8>,
    next_tag: u32,
    pub encrypted: bool,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleWake {
    Exists(u32),
    Expunge,
    Bye,
    Timeout,
    Unsupported,
    Other,
}

impl ImapSession {
    pub fn plain(stream: TcpStream) -> Result<Self, EngineError> {
        apply_timeouts(&stream)?;
        Ok(Self {
            transport: Some(Transport::Plain(stream)),
            read_buf: Vec::new(),
            next_tag: 1,
            encrypted: false,
            fingerprint: None,
        })
    }

    pub fn implicit_tls(
        stream: TcpStream,
        host: &str,
        accept_invalid: bool,
    ) -> Result<Self, EngineError> {
        apply_timeouts(&stream)?;
        let (transport, fingerprint) = wrap_tls(stream, host, accept_invalid)?;
        Ok(Self {
            transport: Some(transport),
            read_buf: Vec::new(),
            next_tag: 1,
            encrypted: true,
            fingerprint,
        })
    }

    pub fn upgrade_starttls(
        &mut self,
        host: &str,
        accept_invalid: bool,
    ) -> Result<(), EngineError> {
        let Some(Transport::Plain(stream)) = self.transport.take() else {
            return Err(EngineError::Tls);
        };
        self.read_buf.clear();
        let (transport, fingerprint) = wrap_tls(stream, host, accept_invalid)?;
        self.transport = Some(transport);
        self.encrypted = true;
        self.fingerprint = fingerprint;
        Ok(())
    }

    fn io(&mut self) -> Result<&mut Transport, EngineError> {
        self.transport.as_mut().ok_or(EngineError::Tls)
    }

    pub fn read_line(&mut self) -> Result<String, EngineError> {
        loop {
            if let Some(pos) = self.read_buf.iter().position(|b| *b == b'\n') {
                let mut line: Vec<u8> = self.read_buf.drain(..=pos).collect();
                if line.ends_with(b"\n") {
                    line.pop();
                }
                if line.ends_with(b"\r") {
                    line.pop();
                }
                return String::from_utf8(line).map_err(|_| EngineError::Server("encoding"));
            }
            if self.read_buf.len() > MAX_LINE {
                return Err(EngineError::Server("line"));
            }
            let mut tmp = [0u8; 2048];
            let n = self.io()?.read(&mut tmp).map_err(map_io)?;
            if n == 0 {
                return Err(EngineError::Network("closed"));
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }

    pub fn write_line(&mut self, line: &str) -> Result<(), EngineError> {
        self.io()?.write_all(line.as_bytes()).map_err(map_io)?;
        self.io()?.write_all(b"\r\n").map_err(map_io)?;
        self.io()?.flush().map_err(map_io)
    }

    pub fn set_io_timeout(&mut self, timeout: Duration) -> Result<(), EngineError> {
        let stream = match self.transport.as_ref().ok_or(EngineError::Tls)? {
            Transport::Plain(s) => s,
            Transport::Tls(s) => s.get_ref(),
        };
        stream.set_read_timeout(Some(timeout)).map_err(map_io)?;
        stream.set_write_timeout(Some(timeout)).map_err(map_io)?;
        Ok(())
    }

    pub fn idle_once(&mut self, wait: Duration) -> Result<IdleWake, EngineError> {
        if wait.is_zero() {
            return Ok(IdleWake::Timeout);
        }
        let tag = format!("A{:04}", self.next_tag);
        self.next_tag += 1;
        self.write_line(&format!("{tag} IDLE"))?;
        loop {
            let line = self.read_line()?;
            if line.starts_with('+') {
                break;
            }
            if line.strip_prefix(&format!("{tag} ")).is_some() {
                return Ok(IdleWake::Unsupported);
            }
        }
        self.set_io_timeout(wait)?;
        let wake = match self.read_line() {
            Ok(line) => parse_idle_event(&line),
            Err(EngineError::Network("timeout")) => IdleWake::Timeout,
            Err(err) => {
                let _ = self.set_io_timeout(IO_TIMEOUT);
                let _ = self.write_line("DONE");
                return Err(err);
            }
        };
        let _ = self.set_io_timeout(IO_TIMEOUT);
        self.write_line("DONE")?;
        loop {
            let line = self.read_line()?;
            if line.strip_prefix(&format!("{tag} ")).is_some() {
                break;
            }
        }
        Ok(wake)
    }

    pub fn command(&mut self, payload: &str) -> Result<CommandResult, EngineError> {
        let tag = format!("A{:04}", self.next_tag);
        self.next_tag += 1;
        self.write_line(&format!("{tag} {payload}"))?;
        let mut untagged = Vec::new();
        loop {
            let line = self.read_line()?;
            if let Some(rest) = line.strip_prefix(&format!("{tag} ")) {
                let ok = rest.starts_with("OK");
                return Ok(CommandResult { ok, tagged: rest.to_string(), untagged });
            }
            if let Some(expanded) = self.maybe_read_literal(&line)? {
                untagged.push(expanded);
            } else {
                untagged.push(line);
            }
        }
    }

    fn maybe_read_literal(&mut self, line: &str) -> Result<Option<String>, EngineError> {
        let Some(start) = line.rfind('{') else {
            return Ok(None);
        };
        if !line.ends_with('}') {
            return Ok(None);
        }
        let Ok(n) = line[start + 1..line.len() - 1].parse::<usize>() else {
            return Ok(None);
        };
        if n > 2 * 1024 * 1024 {
            return Err(EngineError::Server("literal"));
        }
        let mut data = vec![0u8; n];
        let mut filled = 0;
        while filled < n {
            if !self.read_buf.is_empty() {
                let take = (n - filled).min(self.read_buf.len());
                data[filled..filled + take].copy_from_slice(&self.read_buf[..take]);
                self.read_buf.drain(..take);
                filled += take;
                continue;
            }
            let mut tmp = [0u8; 4096];
            let read = self.io()?.read(&mut tmp).map_err(map_io)?;
            if read == 0 {
                return Err(EngineError::Network("closed"));
            }
            self.read_buf.extend_from_slice(&tmp[..read]);
        }
        let literal = String::from_utf8_lossy(&data);
        let rest = self.read_line().unwrap_or_default();
        Ok(Some(format!("{line}\n{literal}{rest}")))
    }

    pub fn greet(&mut self) -> Result<String, EngineError> {
        self.read_line()
    }

    pub fn authenticate_xoauth2(&mut self, ir: &str) -> Result<CommandResult, EngineError> {
        let tag = format!("A{:04}", self.next_tag);
        self.next_tag += 1;
        self.write_line(&format!("{tag} AUTHENTICATE XOAUTH2 {ir}"))?;
        let mut untagged = Vec::new();
        loop {
            let line = self.read_line()?;
            if line.starts_with('+') {
                self.write_line("")?;
                continue;
            }
            if let Some(rest) = line.strip_prefix(&format!("{tag} ")) {
                return Ok(CommandResult {
                    ok: rest.starts_with("OK"),
                    tagged: rest.to_string(),
                    untagged,
                });
            }
            untagged.push(line);
        }
    }

    pub fn command_literal(
        &mut self,
        payload_before: &str,
        literal: &str,
    ) -> Result<CommandResult, EngineError> {
        let tag = format!("A{:04}", self.next_tag);
        self.next_tag += 1;
        self.write_line(&format!("{tag} {payload_before} {{{}}}", literal.len()))?;
        let cont = self.read_line()?;
        if !cont.starts_with('+') {
            return Err(EngineError::Server("literal"));
        }
        self.io()?.write_all(literal.as_bytes()).map_err(map_io)?;
        self.io()?.write_all(b"\r\n").map_err(map_io)?;
        self.io()?.flush().map_err(map_io)?;
        let mut untagged = Vec::new();
        loop {
            let line = self.read_line()?;
            if let Some(rest) = line.strip_prefix(&format!("{tag} ")) {
                return Ok(CommandResult {
                    ok: rest.starts_with("OK"),
                    tagged: rest.to_string(),
                    untagged,
                });
            }
            untagged.push(line);
        }
    }
}

pub struct CommandResult {
    pub ok: bool,
    pub tagged: String,
    pub untagged: Vec<String>,
}

fn wrap_tls(
    mut stream: TcpStream,
    host: &str,
    accept_invalid: bool,
) -> Result<(Transport, Option<String>), EngineError> {
    ensure_crypto();
    let stored = Arc::new(Mutex::new(None));
    let cfg = Arc::new(client_config(accept_invalid, Arc::clone(&stored)));
    let name = ServerName::try_from(host.to_string()).map_err(|_| EngineError::Tls)?;
    let mut conn = ClientConnection::new(cfg, name).map_err(|_| EngineError::Tls)?;
    while conn.is_handshaking() {
        conn.complete_io(&mut stream).map_err(|_| EngineError::Tls)?;
    }
    let fingerprint = conn
        .peer_certificates()
        .and_then(|c| c.first())
        .map(|c| hex_sha256(c.as_ref()))
        .or_else(|| stored.lock().ok().and_then(|g| g.clone()));
    Ok((Transport::Tls(Box::new(StreamOwned::new(conn, stream))), fingerprint))
}

fn apply_timeouts(stream: &TcpStream) -> Result<(), EngineError> {
    stream.set_read_timeout(Some(IO_TIMEOUT)).map_err(map_io)?;
    stream.set_write_timeout(Some(IO_TIMEOUT)).map_err(map_io)?;
    stream.set_nodelay(true).map_err(map_io)?;
    Ok(())
}

pub fn map_io(err: std::io::Error) -> EngineError {
    match err.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => EngineError::timeout(),
        std::io::ErrorKind::ConnectionRefused => EngineError::Network("refused"),
        std::io::ErrorKind::NotFound => EngineError::Network("dns"),
        _ => EngineError::Network("io"),
    }
}

#[must_use]
pub fn imap_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[must_use]
pub fn parse_list_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("* LIST ")?.trim_start();
    let after_flags = if rest.starts_with('(') {
        let end = rest.find(')')?;
        rest[end + 1..].trim_start()
    } else {
        rest
    };
    let after_delim = skip_imap_string(after_flags)?.trim_start();
    parse_imap_string(after_delim)
}

fn skip_imap_string(s: &str) -> Option<&str> {
    if s.is_empty() {
        return None;
    }
    if s.starts_with("NIL") {
        return Some(s.get(3..).unwrap_or(""));
    }
    if s.starts_with('"') {
        let mut escaped = false;
        for (i, c) in s.char_indices().skip(1) {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                return Some(&s[i + 1..]);
            }
        }
        return None;
    }
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    Some(&s[end..])
}

#[must_use]
pub fn parse_list_flags(line: &str) -> Vec<String> {
    let rest = match line.strip_prefix("* LIST ") {
        Some(r) => r.trim_start(),
        None => return Vec::new(),
    };
    let Some(inner) = rest.strip_prefix('(') else {
        return Vec::new();
    };
    let Some(end) = inner.find(')') else {
        return Vec::new();
    };
    inner[..end]
        .split_whitespace()
        .map(|f| f.trim_start_matches('\\').to_string())
        .filter(|f| !f.is_empty())
        .collect()
}

fn parse_imap_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.starts_with('"') {
        let mut out = String::new();
        let mut escaped = false;
        for c in s.chars().skip(1) {
            if escaped {
                out.push(c);
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                return Some(out);
            }
            out.push(c);
        }
        return None;
    }
    Some(s.split_whitespace().next()?.to_string())
}

#[must_use]
pub fn capability_has(untagged: &[String], token: &str) -> bool {
    let needle = token.to_ascii_uppercase();
    untagged.iter().any(|l| l.to_ascii_uppercase().split_whitespace().any(|w| w == needle))
}

#[must_use]
pub fn parse_idle_event(line: &str) -> IdleWake {
    let u = line.to_ascii_uppercase();
    if u.starts_with("* BYE") {
        return IdleWake::Bye;
    }
    let Some(rest) = u.strip_prefix("* ") else {
        return IdleWake::Other;
    };
    let mut parts = rest.split_whitespace();
    let n = parts.next();
    match parts.next() {
        Some("EXISTS") => IdleWake::Exists(n.and_then(|s| s.parse().ok()).unwrap_or(0)),
        Some("EXPUNGE") => IdleWake::Expunge,
        _ => IdleWake::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_special_chars() {
        assert_eq!(imap_quote(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn parses_list_quoted_and_atom() {
        assert_eq!(
            parse_list_name(r#"* LIST (\HasNoChildren) "/" INBOX"#).as_deref(),
            Some("INBOX")
        );
        assert_eq!(
            parse_list_name(r#"* LIST (\Sent) "/" "Sent Messages""#).as_deref(),
            Some("Sent Messages")
        );
    }

    #[test]
    fn parses_idle_exists_and_expunge() {
        assert_eq!(parse_idle_event("* 2 EXISTS"), IdleWake::Exists(2));
        assert_eq!(parse_idle_event("* 1 EXPUNGE"), IdleWake::Expunge);
        assert_eq!(parse_idle_event("* BYE IMAP4rev1 Server logging out"), IdleWake::Bye);
        assert_eq!(parse_idle_event("* 1 RECENT"), IdleWake::Other);
    }
}
