use thiserror::Error;

/// 六端统一错误模型，UI 按变体映射本地化文案。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EngineError {
    #[error("authentication failed")]
    AuthFailed,
    #[error("authentication expired: {0}")]
    AuthExpired(&'static str),
    #[error("network: {0}")]
    Network(&'static str),
    #[error("tls error")]
    Tls,
    #[error("server: {0}")]
    Server(&'static str),
    #[error("quota exceeded")]
    QuotaExceeded,
    #[error("attachment too large (limit {limit_mb} MB)")]
    AttachmentTooLarge { limit_mb: u32 },
    #[error("rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },
    #[error("not found")]
    NotFound,
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("storage: {0}")]
    Storage(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("unknown (desensitized)")]
    Unknown,
}

impl EngineError {
    #[must_use]
    pub fn offline() -> Self {
        Self::Network("offline")
    }

    #[must_use]
    pub fn timeout() -> Self {
        Self::Network("timeout")
    }
}
