use crate::error::EngineError;
use crate::ids::AccountId;
use crate::oauth::{MailAuth, xoauth2_ir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountMode {
    Standard,
    Enhanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountState {
    Online,
    Syncing,
    Error,
    AuthRequired,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    OAuth2,
    AuthCode,
    AppPassword,
    Password,
    ApiToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncScope {
    All,
    #[default]
    Last30Days,
    HeadersOnly {
        limit: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub email: String,
    pub display_name: String,
    pub provider_id: String,
    pub mode: AccountMode,
    pub color: String,
    pub sync_scope: SyncScope,
    pub state: AccountState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddAccountRequest {
    pub email: String,
    pub display_name: Option<String>,
    pub password: Option<String>,
    pub auth_kind: Option<AuthKind>,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    /// SMTP 587 STARTTLS（Outlook/iCloud）。默认隐式 TLS 465。
    #[serde(default)]
    pub smtp_starttls: bool,
    pub api_base: Option<String>,
    pub api_token: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub username: Option<String>,
    /// 默认隐式 TLS（993）。`true` 时先明文问候再 STARTTLS，失败不断开后登录。
    #[serde(default)]
    pub imap_starttls: bool,
    /// 自签名 TOFU / 本地测试邮局。默认关闭。
    #[serde(default)]
    pub accept_invalid_certs: bool,
    #[serde(default)]
    pub sync_scope: SyncScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub starttls: bool,
    pub accept_invalid_certs: bool,
    pub username: String,
}

/// 添加账号向导用的服务商摘要（与 `providers.json` 对齐，不含密钥）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub domains: Vec<String>,
    pub display_name_zh: String,
    pub display_name_en: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: bool,
    pub auth_kind: String,
    #[serde(default)]
    pub help_url: Option<String>,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub token_url: Option<String>,
    #[serde(default)]
    pub auth_url: Option<String>,
    #[serde(default)]
    pub client_id_env: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

impl AddAccountRequest {
    #[must_use]
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            display_name: None,
            password: None,
            auth_kind: None,
            imap_host: None,
            imap_port: None,
            smtp_host: None,
            smtp_port: None,
            smtp_starttls: false,
            api_base: None,
            api_token: None,
            access_token: None,
            refresh_token: None,
            username: None,
            imap_starttls: false,
            accept_invalid_certs: false,
            sync_scope: SyncScope::default(),
        }
    }

    #[must_use]
    pub fn uses_xoauth2(&self) -> bool {
        self.access_token.as_deref().is_some_and(|t| !t.is_empty())
    }

    #[must_use]
    pub fn has_mail_secret(&self) -> bool {
        self.uses_xoauth2() || self.password.as_deref().is_some_and(|p| !p.is_empty())
    }

    pub fn mail_auth(&self) -> Result<MailAuth, EngineError> {
        let user = self.username.as_deref().unwrap_or(self.email.as_str()).to_string();
        if let Some(token) = self.access_token.as_deref().filter(|s| !s.is_empty()) {
            return Ok(MailAuth::Xoauth2 { ir: xoauth2_ir(&user, token), user });
        }
        let password = self
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(EngineError::AuthFailed)?
            .to_string();
        Ok(MailAuth::Login { user, password })
    }
}
