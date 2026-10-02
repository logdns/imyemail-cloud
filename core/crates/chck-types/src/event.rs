use crate::AccountState;
use crate::ids::{AccountId, DraftId, FolderId};
use crate::message::{MessageSummary, OutboxState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineEvent {
    AccountStateChanged { account_id: AccountId, state: AccountState },
    NewMessages { account_id: AccountId, folder_id: FolderId, count: u32, top: Vec<MessageSummary> },
    MessageFlagsChanged { ids: Vec<crate::ids::MessageId>, flags: crate::message::Flags },
    SyncProgress { account_id: AccountId, folder_id: FolderId, done: u32, total: u32 },
    SendQueueChanged { draft_id: DraftId, state: OutboxState, error: Option<String> },
}
