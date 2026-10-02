use crate::ids::{AccountId, FolderId, MessageId, ThreadId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Flags {
    pub seen: bool,
    pub flagged: bool,
    pub answered: bool,
    pub draft: bool,
    pub deleted: bool,
}

impl Flags {
    const SEEN: i64 = 1;
    const FLAGGED: i64 = 2;
    const ANSWERED: i64 = 4;
    const DRAFT: i64 = 8;
    const DELETED: i64 = 16;

    #[must_use]
    pub fn to_bits(self) -> i64 {
        let mut bits = 0;
        if self.seen {
            bits |= Self::SEEN;
        }
        if self.flagged {
            bits |= Self::FLAGGED;
        }
        if self.answered {
            bits |= Self::ANSWERED;
        }
        if self.draft {
            bits |= Self::DRAFT;
        }
        if self.deleted {
            bits |= Self::DELETED;
        }
        bits
    }

    #[must_use]
    pub fn from_bits(bits: i64) -> Self {
        Self {
            seen: bits & Self::SEEN != 0,
            flagged: bits & Self::FLAGGED != 0,
            answered: bits & Self::ANSWERED != 0,
            draft: bits & Self::DRAFT != 0,
            deleted: bits & Self::DELETED != 0,
        }
    }

    #[must_use]
    pub fn from_imap(flags: &str) -> Self {
        let upper = flags.to_ascii_uppercase();
        Self {
            seen: upper.contains("\\SEEN"),
            flagged: upper.contains("\\FLAGGED"),
            answered: upper.contains("\\ANSWERED"),
            draft: upper.contains("\\DRAFT"),
            deleted: upper.contains("\\DELETED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyState {
    HeadersOnly,
    Snippet,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSummary {
    pub id: MessageId,
    pub account_id: AccountId,
    pub folder_id: FolderId,
    pub thread_id: Option<ThreadId>,
    pub subject: String,
    pub from: Vec<Address>,
    #[serde(default)]
    pub to: Vec<Address>,
    pub date_unix: i64,
    pub snippet: String,
    pub flags: Flags,
    pub has_attachments: bool,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageBody {
    pub id: MessageId,
    pub html_sanitized: String,
    pub text: String,
    pub remote_blocked: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutboxState {
    Draft,
    Queued,
    Sending,
    Sent,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendQueueItem {
    pub draft_id: crate::ids::DraftId,
    pub account_id: AccountId,
    pub state: OutboxState,
    pub retry_count: u32,
    pub error: Option<String>,
}
