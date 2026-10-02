use crate::ids::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub account_id: AccountId,
    pub name: String,
    pub enabled: bool,
    pub definition: RuleDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuleDef {
    #[serde(default)]
    pub match_all: bool,
    #[serde(default)]
    pub conditions: Vec<RuleCondition>,
    #[serde(default)]
    pub actions: Vec<RuleAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "camelCase")]
pub enum RuleCondition {
    From { contains: String },
    To { contains: String },
    Subject { contains: String },
    Body { contains: String },
    HasAttachment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RuleAction {
    Flag { seen: Option<bool>, flagged: Option<bool> },
    Delete,
    Move { folder_path: String },
    Label { name: String },
}

impl RuleDef {
    #[must_use]
    pub fn matches(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        has_attachment: bool,
    ) -> bool {
        if self.conditions.is_empty() {
            return false;
        }
        let check = |c: &RuleCondition| match c {
            RuleCondition::From { contains } => contains_ci(from, contains),
            RuleCondition::To { contains } => contains_ci(to, contains),
            RuleCondition::Subject { contains } => contains_ci(subject, contains),
            RuleCondition::Body { contains } => contains_ci(body, contains),
            RuleCondition::HasAttachment => has_attachment,
        };
        if self.match_all {
            self.conditions.iter().all(check)
        } else {
            self.conditions.iter().any(check)
        }
    }
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_any_match() {
        let def = RuleDef {
            match_all: false,
            conditions: vec![RuleCondition::Subject { contains: "invoice".into() }],
            actions: vec![RuleAction::Flag { seen: Some(true), flagged: None }],
        };
        assert!(def.matches("a@b.com", "", "INVOICE 12", "", false));
        assert!(!def.matches("a@b.com", "", "hello", "", false));
    }
}
