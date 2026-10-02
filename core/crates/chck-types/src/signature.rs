use crate::ids::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    #[serde(default)]
    pub id: String,
    pub account_id: AccountId,
    pub name: String,
    pub body: String,
    #[serde(default)]
    pub is_default: bool,
}
