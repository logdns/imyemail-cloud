use crate::ids::{AccountId, FolderId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Junk,
    Archive,
    Flagged,
    All,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub account_id: AccountId,
    pub remote_name: String,
    pub path: String,
    pub role: FolderRole,
    pub unread_count: u32,
    pub total_count: u32,
}

impl FolderRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Sent => "sent",
            Self::Drafts => "drafts",
            Self::Trash => "trash",
            Self::Junk => "junk",
            Self::Archive => "archive",
            Self::Flagged => "flagged",
            Self::All => "all",
            Self::Custom => "custom",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "inbox" => Self::Inbox,
            "sent" => Self::Sent,
            "drafts" => Self::Drafts,
            "trash" => Self::Trash,
            "junk" => Self::Junk,
            "archive" => Self::Archive,
            "flagged" => Self::Flagged,
            "all" => Self::All,
            _ => Self::Custom,
        }
    }
}
