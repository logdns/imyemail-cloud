use crate::ids::{AccountId, FolderId, MessageId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    Local,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub account_id: Option<AccountId>,
    pub folder_id: Option<FolderId>,
    pub has_attachment: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub message_id: MessageId,
    pub source: SearchSource,
    pub rank: i32,
}
