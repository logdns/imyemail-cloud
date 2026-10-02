use crate::account::{Account, AddAccountRequest, ProviderInfo};
use crate::contact::{CalendarInvite, Contact, Rsvp};
use crate::error::EngineError;
use crate::event::EngineEvent;
use crate::folder::Folder;
use crate::ids::{AccountId, AttachmentId, DraftId, FolderId, MessageId, ThreadId};
use crate::message::{Flags, MessageBody, MessageSummary, SendQueueItem};
use crate::oauth::OAuthStart;
use crate::probe::ConnectProbe;
use crate::rules::Rule;
use crate::search::{SearchHit, SearchQuery};
use crate::signature::Signature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub offset: u32,
    pub limit: u32,
}

impl Default for Page {
    fn default() -> Self {
        Self { offset: 0, limit: 50 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendRequest {
    pub account_id: AccountId,
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body_text: String,
    #[serde(default)]
    pub body_html: Option<String>,
    #[serde(default = "default_undo")]
    pub undo_window_secs: u32,
}

fn default_undo() -> u32 {
    10
}

impl Default for SendRequest {
    fn default() -> Self {
        Self {
            account_id: AccountId::from(""),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            body_text: String::new(),
            body_html: None,
            undo_window_secs: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentHandle {
    pub id: AttachmentId,
    pub name: String,
    pub mime: String,
    pub size: u64,
    #[serde(default)]
    pub cid: Option<String>,
}

/// 各端统一入口。S1：`test_account` 返回逐项探测结果；其余切片仍可 `Unsupported`。
pub trait MailEngine: Send + Sync {
    fn add_account(&self, req: AddAccountRequest) -> Result<Account, EngineError>;
    fn update_account(&self, account: Account) -> Result<Account, EngineError>;
    fn remove_account(&self, id: &AccountId) -> Result<(), EngineError>;
    fn test_account(&self, req: &AddAccountRequest) -> Result<ConnectProbe, EngineError>;
    fn list_accounts(&self) -> Result<Vec<Account>, EngineError>;
    fn list_providers(&self) -> Result<Vec<ProviderInfo>, EngineError>;

    fn list_folders(&self, account_id: &AccountId) -> Result<Vec<Folder>, EngineError>;
    fn sync_folder(&self, folder_id: &FolderId) -> Result<(), EngineError>;

    fn list_messages(
        &self,
        folder_id: &FolderId,
        page: Page,
    ) -> Result<Vec<MessageSummary>, EngineError>;
    fn unified_inbox(&self, page: Page) -> Result<Vec<MessageSummary>, EngineError>;
    fn list_thread(&self, thread_id: &ThreadId) -> Result<Vec<MessageSummary>, EngineError>;
    fn body(&self, id: &MessageId) -> Result<MessageBody, EngineError>;
    fn set_flags(&self, id: &MessageId, flags: Flags) -> Result<(), EngineError>;
    fn move_message(&self, id: &MessageId, dest: &FolderId) -> Result<(), EngineError>;
    fn delete_message(&self, id: &MessageId) -> Result<(), EngineError>;
    fn bulk_set_flags(&self, ids: &[MessageId], flags: Flags) -> Result<(), EngineError>;
    fn bulk_delete(&self, ids: &[MessageId]) -> Result<(), EngineError>;

    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, EngineError>;
    fn probe_enhanced(&self, account_id: &AccountId) -> Result<bool, EngineError>;

    fn save_draft(&self, req: SendRequest) -> Result<DraftId, EngineError>;
    fn send(&self, req: SendRequest) -> Result<DraftId, EngineError>;
    fn schedule_send(&self, req: SendRequest, send_at_unix: i64) -> Result<DraftId, EngineError>;
    fn undo_send(&self, draft_id: &DraftId) -> Result<(), EngineError>;
    fn outbox(&self) -> Result<Vec<SendQueueItem>, EngineError>;
    fn flush_due_sends(&self, now_unix: i64) -> Result<u32, EngineError>;

    fn add_rule(&self, rule: Rule) -> Result<Rule, EngineError>;
    fn list_rules(&self, account_id: &AccountId) -> Result<Vec<Rule>, EngineError>;
    fn remove_rule(&self, rule_id: &str) -> Result<(), EngineError>;
    fn apply_rules(&self, message_id: &MessageId) -> Result<u32, EngineError>;

    fn snooze(&self, id: &MessageId, until_unix: i64) -> Result<(), EngineError>;
    fn wake_snoozed(&self, now_unix: i64) -> Result<u32, EngineError>;

    fn save_signature(&self, signature: Signature) -> Result<Signature, EngineError>;
    fn list_signatures(&self, account_id: &AccountId) -> Result<Vec<Signature>, EngineError>;
    fn remove_signature(&self, signature_id: &str) -> Result<(), EngineError>;

    fn list_attachments(
        &self,
        message_id: &MessageId,
    ) -> Result<Vec<AttachmentHandle>, EngineError>;
    fn fetch_attachment(&self, id: &AttachmentId) -> Result<Vec<u8>, EngineError>;

    fn upsert_contact(&self, contact: Contact) -> Result<Contact, EngineError>;
    fn list_contacts(&self, account_id: Option<&AccountId>) -> Result<Vec<Contact>, EngineError>;
    fn set_vip(&self, contact_id: &str, vip: bool) -> Result<(), EngineError>;
    fn import_vcard(&self, account_id: Option<&AccountId>, vcard: &str)
    -> Result<u32, EngineError>;
    fn import_csv(&self, account_id: Option<&AccountId>, csv: &str) -> Result<u32, EngineError>;

    fn parse_invite(&self, ics: &str) -> Result<CalendarInvite, EngineError>;
    fn rsvp_invite(&self, invite: &CalendarInvite, rsvp: Rsvp) -> Result<String, EngineError>;

    fn diagnose(&self, req: &AddAccountRequest) -> Result<ConnectProbe, EngineError>;
    fn export_backup(&self) -> Result<String, EngineError>;
    fn import_backup(&self, json: &str) -> Result<u32, EngineError>;

    fn start_oauth(&self, email: &str) -> Result<OAuthStart, EngineError>;
    fn complete_oauth(
        &self,
        email: &str,
        code: &str,
        verifier: &str,
        redirect_uri: Option<&str>,
    ) -> Result<AddAccountRequest, EngineError>;

    fn poll_events(&self) -> Vec<EngineEvent>;

    fn tick(
        &self,
        account_id: Option<&AccountId>,
        idle: bool,
        idle_wait_ms: Option<u64>,
    ) -> Result<Vec<SyncTick>, EngineError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncState {
    Idle,
    Connecting,
    Discovering,
    SyncFolders,
    SyncFolder,
    Done,
    Idling,
    Backoff,
}

impl SyncState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Connecting => "connecting",
            Self::Discovering => "discovering",
            Self::SyncFolders => "syncFolders",
            Self::SyncFolder => "syncFolder",
            Self::Done => "done",
            Self::Idling => "idling",
            Self::Backoff => "backoff",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTick {
    pub account_id: Option<AccountId>,
    pub state: SyncState,
    pub folder_id: Option<FolderId>,
    pub supports_idle: bool,
}
