//! Webhook 校验与静默推送组装。推送体不含主题/发件人/正文。

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailReceived {
    pub account_id: String,
    pub folder: String,
    #[serde(default)]
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilentPush {
    pub account_id: String,
    pub folder: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub token: String,
    pub platform: Platform,
    pub account_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Apns,
    Fcm,
}

pub fn verify_signature(secret: &[u8], body: &[u8], hex_sig: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());
    expected.eq_ignore_ascii_case(hex_sig.trim())
}

pub fn assemble_push(event: &MailReceived) -> SilentPush {
    SilentPush {
        account_id: event.account_id.clone(),
        folder: event.folder.clone(),
        count: event.count,
    }
}

pub fn parse_event(body: &str) -> Result<MailReceived, String> {
    serde_json::from_str(body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_roundtrip() {
        let body = br#"{"account_id":"a1","folder":"INBOX","count":1}"#;
        let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(b"secret", body, &sig));
        assert!(!verify_signature(b"secret", body, "deadbeef"));
    }

    #[test]
    fn push_has_no_subject() {
        let ev = parse_event(r#"{"account_id":"a1","folder":"INBOX","count":2}"#).unwrap();
        let p = assemble_push(&ev);
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("subject"));
        assert!(!json.contains("from"));
        assert!(json.contains("a1"));
    }
}
