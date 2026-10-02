//! 服务商预设表。内置于二进制，可由 `https://imy.email` 提供兼容更新。

use serde::{Deserialize, Serialize};

pub const BUILTIN_JSON: &str = include_str!("../data/providers.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCatalog {
    pub version: u32,
    pub providers: Vec<Provider>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub domains: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: DisplayName,
    pub imap: Endpoint,
    pub smtp: Endpoint,
    pub auth: AuthPreset,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayName {
    pub zh: String,
    pub en: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub tls: TlsMode,
    #[serde(default)]
    pub alt: Option<AltEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltEndpoint {
    pub port: u16,
    pub tls: TlsMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TlsMode {
    Tls,
    Starttls,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthPreset {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "helpUrl", default)]
    pub help_url: Option<String>,
    #[serde(rename = "tokenUrl", default)]
    pub token_url: Option<String>,
    #[serde(rename = "authUrl", default)]
    pub auth_url: Option<String>,
    #[serde(rename = "clientIdEnv", default)]
    pub client_id_env: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(rename = "redirectUri", default)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Limits {
    #[serde(rename = "attachmentMB", default)]
    pub attachment_mb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub condstore: bool,
    #[serde(default)]
    pub idle: bool,
}

impl ProviderCatalog {
    /// 解析内置预设。启动失败即视为构建错误。
    pub fn builtin() -> Self {
        serde_json::from_str(BUILTIN_JSON).expect("providers.json must parse")
    }

    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[must_use]
    pub fn by_email(&self, email: &str) -> Option<&Provider> {
        let domain = email.rsplit('@').next()?.to_ascii_lowercase();
        self.providers.iter().find(|p| p.domains.iter().any(|d| d.eq_ignore_ascii_case(&domain)))
    }

    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }
}

pub const QUIRK_IMAP_ID: &str = "imap-id";
pub const QUIRK_RATE_LIMIT: &str = "rate-limit";
pub const QUIRK_SENT_APPEND: &str = "sent-append";
pub const QUIRK_GMAIL_LABELS: &str = "gmail-labels";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_twenty_named_providers() {
        let cat = ProviderCatalog::builtin();
        assert!(cat.version >= 1);
        assert_eq!(cat.providers.len(), 20);
    }

    #[test]
    fn qq_requires_imap_id_and_authcode() {
        let cat = ProviderCatalog::builtin();
        let qq = cat.by_email("user@qq.com").expect("qq");
        assert_eq!(qq.id, "qq");
        assert_eq!(qq.auth.kind, "authcode");
        assert!(qq.quirks.iter().any(|q| q == QUIRK_IMAP_ID));
        assert_eq!(qq.imap.port, 993);
        assert_eq!(qq.smtp.port, 465);
    }

    #[test]
    fn gmail_and_googlemail_share_preset() {
        let cat = ProviderCatalog::builtin();
        assert_eq!(cat.by_email("a@gmail.com").unwrap().id, "gmail");
        assert_eq!(cat.by_email("a@googlemail.com").unwrap().id, "gmail");
        assert_eq!(cat.by_email("a@gmail.com").unwrap().auth.kind, "oauth2");
        let gmail = cat.by_email("a@gmail.com").unwrap();
        assert_eq!(gmail.auth.token_url.as_deref(), Some("https://oauth2.googleapis.com/token"));
        assert!(gmail.auth.scopes.iter().any(|s| s.contains("mail.google.com")));
    }

    #[test]
    fn netease_family_requires_imap_id() {
        let cat = ProviderCatalog::builtin();
        for addr in ["a@163.com", "a@126.com", "a@yeah.net"] {
            let p = cat.by_email(addr).unwrap();
            assert!(p.quirks.iter().any(|q| q == QUIRK_IMAP_ID), "{addr}");
            assert_eq!(p.auth.kind, "authcode");
        }
    }

    #[test]
    fn unknown_domain_is_none() {
        let cat = ProviderCatalog::builtin();
        assert!(cat.by_email("me@example.com").is_none());
    }

    #[test]
    fn outlook_aliases() {
        let cat = ProviderCatalog::builtin();
        for addr in ["a@outlook.com", "a@hotmail.com", "a@live.com", "a@msn.com"] {
            assert_eq!(cat.by_email(addr).unwrap().id, "outlook");
        }
    }
}
