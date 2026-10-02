//! 领域模型、统一错误、事件流与 `MailEngine` 契约。
//!
//! UI 层只依赖本 crate 的类型 + `chck-engine` 门面，不接触协议细节。

mod account;
mod contact;
mod engine;
mod error;
mod event;
mod folder;
mod ids;
mod message;
mod oauth;
mod probe;
mod rules;
mod search;
mod signature;

pub use account::{
    Account, AccountMode, AccountState, AddAccountRequest, AuthKind, ImapConfig, ProviderInfo,
    SyncScope,
};
pub use contact::{CalendarInvite, Contact, Rsvp, parse_csv, parse_ics, parse_vcard, rsvp_ics};
pub use engine::{AttachmentHandle, MailEngine, Page, SendRequest, SyncState, SyncTick};
pub use error::EngineError;
pub use event::EngineEvent;
pub use folder::{Folder, FolderRole};
pub use ids::{AccountId, AttachmentId, DraftId, FolderId, MessageId, ThreadId};
pub use message::{
    Address, BodyState, Flags, MessageBody, MessageSummary, OutboxState, SendQueueItem,
};
pub use oauth::{
    MailAuth, OAuthStart, OAuthTokenSet, Pkce, authorize_url, pkce_s256, pkce_verifier, xoauth2_ir,
};
pub use probe::{ConnectProbe, ProbeKind, ProbeStatus, ProbeStep};
pub use rules::{Rule, RuleAction, RuleCondition, RuleDef};
pub use search::{SearchHit, SearchQuery, SearchSource};
pub use signature::Signature;
