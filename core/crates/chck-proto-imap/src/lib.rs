//! IMAP：连接探测 + 文件夹/头同步 + IDLE / NOOP。
//! 明文 IMAP 禁止；LOGIN / AUTHENTICATE XOAUTH2 仅在 TLS 之后。QQ/网易 quirks 在认证前发 IMAP ID。

mod connect;
mod fetch;
mod folders;
mod headers;
mod probe;
mod session;
mod tls;

pub use connect::{authenticate, connect_and_login, connect_endpoint};
pub use fetch::{EnvelopeFetch, parse_fetch_untagged};
pub use folders::{RemoteFolder, folder_role};
pub use headers::{ParsedHeaders, parse_headers};
pub use probe::{Endpoint, probe, resolved_endpoint};
pub use session::{
    CommandResult, IdleWake, ImapSession, imap_quote, parse_idle_event, parse_list_flags,
    parse_list_name,
};

use chck_providers::Provider;
use chck_types::{AddAccountRequest, ConnectProbe, EngineError};
use session::capability_has;

pub struct ImapClient;

impl ImapClient {
    pub fn test_connect(
        req: &AddAccountRequest,
        provider: Option<&Provider>,
    ) -> Result<ConnectProbe, EngineError> {
        probe(req, provider)
    }

    pub fn login(
        req: &AddAccountRequest,
        provider: Option<&Provider>,
    ) -> Result<ImapSession, EngineError> {
        connect_and_login(req, provider)
    }
}

impl ImapSession {
    pub fn list_folders(&mut self) -> Result<Vec<RemoteFolder>, EngineError> {
        let list = self.command("LIST \"\" \"*\"")?;
        if !list.ok {
            return Err(EngineError::Server("list"));
        }
        Ok(list
            .untagged
            .iter()
            .filter_map(|line| {
                let path = parse_list_name(line)?;
                let flags = parse_list_flags(line);
                let role = folder_role(&flags, &path);
                Some(RemoteFolder { path, delimiter: "/".into(), flags, role })
            })
            .collect())
    }

    pub fn select(&mut self, mailbox: &str) -> Result<SelectStatus, EngineError> {
        let res = self.command(&format!("SELECT {}", imap_quote(mailbox)))?;
        if !res.ok {
            return Err(EngineError::Server("select"));
        }
        let mut status = SelectStatus::default();
        for line in &res.untagged {
            let u = line.to_ascii_uppercase();
            if let Some(n) = u.strip_prefix("* ")
                && let Some(rest) = n.strip_suffix(" EXISTS")
            {
                status.exists = rest.trim().parse().unwrap_or(0);
            }
            if let Some(uid) = extract_after(line, "[UIDVALIDITY ") {
                status.uidvalidity = uid;
            }
            if let Some(uid) = extract_after(line, "[UIDNEXT ") {
                status.uidnext = uid;
            }
            if u.contains("HIGHESTMODSEQ")
                && let Some(n) = extract_after(line, "HIGHESTMODSEQ ")
            {
                status.highestmodseq = Some(n);
                status.condstore = true;
            }
        }
        let caps = self.command("CAPABILITY")?;
        if capability_has(&caps.untagged, "CONDSTORE") {
            status.condstore = true;
        }
        Ok(status)
    }

    pub fn fetch_headers_since(
        &mut self,
        from_uid: u32,
        limit: u32,
    ) -> Result<Vec<EnvelopeFetch>, EngineError> {
        let range = if from_uid <= 1 { "1:*".to_string() } else { format!("{from_uid}:*") };
        let cmd = format!(
            "UID FETCH {range} (UID FLAGS RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM TO CC SUBJECT DATE MESSAGE-ID IN-REPLY-TO CONTENT-TYPE CONTENT-DISPOSITION)])"
        );
        let res = self.command(&cmd)?;
        if !res.ok {
            return Err(EngineError::Server("fetch"));
        }
        let mut items = parse_fetch_untagged(&res);
        items.retain(|i| i.uid >= from_uid);
        items.sort_by_key(|i| i.uid);
        if items.len() > limit as usize {
            let skip = items.len() - limit as usize;
            items = items.split_off(skip);
        }
        Ok(items)
    }

    pub fn fetch_rfc822(&mut self, uid: u32) -> Result<String, EngineError> {
        let res = self.command(&format!("UID FETCH {uid} (BODY.PEEK[])"))?;
        if !res.ok {
            return Err(EngineError::Server("fetch-body"));
        }
        let blob = res.untagged.join("\n");
        extract_rfc822(&blob).ok_or(EngineError::NotFound)
    }

    pub fn uid_store_flags(&mut self, uid: u32, flags: &str) -> Result<(), EngineError> {
        let res = self.command(&format!("UID STORE {uid} FLAGS.SILENT ({flags})"))?;
        if res.ok { Ok(()) } else { Err(EngineError::Server("store")) }
    }

    pub fn uid_copy(&mut self, uid: u32, dest: &str) -> Result<(), EngineError> {
        let res = self.command(&format!("UID COPY {uid} {}", imap_quote(dest)))?;
        if res.ok { Ok(()) } else { Err(EngineError::Server("copy")) }
    }

    pub fn require_uidplus(&mut self) -> Result<(), EngineError> {
        let caps = self.command("CAPABILITY")?;
        if !caps.ok || !session::capability_has(&caps.untagged, "UIDPLUS") {
            return Err(EngineError::Unsupported("operation requires UIDPLUS"));
        }
        Ok(())
    }

    /// Copy one message and obtain its server-assigned destination UID.
    pub fn uid_copy_mapped(&mut self, uid: u32, dest: &str) -> Result<(u32, u32), EngineError> {
        self.require_uidplus()?;
        let res = self.command(&format!("UID COPY {uid} {}", imap_quote(dest)))?;
        if !res.ok {
            return Err(EngineError::Server("copy"));
        }
        let upper = res.tagged.to_ascii_uppercase();
        let mapped = upper.split("[COPYUID ").nth(1).and_then(|s| s.split(']').next());
        let fields: Vec<_> = mapped.unwrap_or("").split_whitespace().collect();
        if fields.len() != 3 || fields[1].parse::<u32>().ok() != Some(uid) {
            return Err(EngineError::Server("copy UID mapping missing; source retained"));
        }
        let validity = fields[0]
            .parse::<u32>()
            .ok()
            .filter(|v| *v != 0)
            .ok_or(EngineError::Server("copy UIDVALIDITY invalid; source retained"))?;
        let destination_uid = fields[2]
            .parse::<u32>()
            .ok()
            .filter(|uid| *uid != 0)
            .ok_or(EngineError::Server("copy UID mapping invalid; source retained"))?;
        Ok((validity, destination_uid))
    }

    pub fn uid_delete_only(&mut self, uid: u32) -> Result<(), EngineError> {
        let marked = self.command(&format!("UID STORE {uid} +FLAGS.SILENT (\\Deleted)"))?;
        if !marked.ok {
            return Err(EngineError::Server("store"));
        }
        let res = self.command(&format!("UID EXPUNGE {uid}"))?;
        if res.ok { Ok(()) } else { Err(EngineError::Server("uid expunge")) }
    }

    pub fn expunge(&mut self) -> Result<(), EngineError> {
        let res = self.command("EXPUNGE")?;
        if res.ok { Ok(()) } else { Err(EngineError::Server("expunge")) }
    }

    pub fn append_rfc822(&mut self, mailbox: &str, rfc822: &str) -> Result<(), EngineError> {
        let payload = format!("APPEND {} (\\Seen)", imap_quote(mailbox));
        let res = self.command_literal(&payload, rfc822)?;
        if res.ok { Ok(()) } else { Err(EngineError::Server("append")) }
    }

    pub fn logout(&mut self) {
        let _ = self.command("LOGOUT");
    }

    pub fn capability_tokens(&mut self) -> Result<Vec<String>, EngineError> {
        let caps = self.command("CAPABILITY")?;
        if !caps.ok {
            return Err(EngineError::Server("capability"));
        }
        Ok(caps
            .untagged
            .iter()
            .filter_map(|line| line.strip_prefix("* CAPABILITY "))
            .flat_map(|rest| rest.split_whitespace())
            .map(str::to_ascii_uppercase)
            .collect())
    }

    pub fn supports_idle(&mut self) -> Result<bool, EngineError> {
        Ok(self.capability_tokens()?.iter().any(|t| t == "IDLE"))
    }

    pub fn noop(&mut self) -> Result<CommandResult, EngineError> {
        let res = self.command("NOOP")?;
        if res.ok { Ok(res) } else { Err(EngineError::Server("noop")) }
    }

    pub fn idle_wait(&mut self, wait: std::time::Duration) -> Result<IdleWake, EngineError> {
        self.idle_once(wait)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectStatus {
    pub exists: u32,
    pub uidvalidity: u32,
    pub uidnext: u32,
    pub highestmodseq: Option<u32>,
    pub condstore: bool,
}

fn extract_after(line: &str, key: &str) -> Option<u32> {
    let pos = line.to_ascii_uppercase().find(&key.to_ascii_uppercase())?;
    line[pos + key.len()..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn extract_rfc822(blob: &str) -> Option<String> {
    crate::fetch::extract_first_literal(blob)
}
