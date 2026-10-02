use chck_types::FolderRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFolder {
    pub path: String,
    pub delimiter: String,
    pub flags: Vec<String>,
    pub role: FolderRole,
}

#[must_use]
pub fn folder_role(flags: &[String], name: &str) -> FolderRole {
    let flag_hit = flags.iter().find_map(|f| match f.to_ascii_lowercase().as_str() {
        "inbox" => Some(FolderRole::Inbox),
        "sent" => Some(FolderRole::Sent),
        "drafts" => Some(FolderRole::Drafts),
        "trash" => Some(FolderRole::Trash),
        "junk" => Some(FolderRole::Junk),
        "archive" => Some(FolderRole::Archive),
        "flagged" | "starred" => Some(FolderRole::Flagged),
        "all" | "allmail" => Some(FolderRole::All),
        _ => None,
    });
    if let Some(role) = flag_hit {
        return role;
    }
    match normalize_name(name).as_str() {
        "inbox" => FolderRole::Inbox,
        "sent" | "sent messages" | "sent items" | "已发送" | "送信済み" | "отправленные" => {
            FolderRole::Sent
        }
        "drafts" | "draft" | "草稿" => FolderRole::Drafts,
        "trash" | "deleted" | "deleted messages" | "已删除" | "废纸篓" => FolderRole::Trash,
        "junk" | "spam" | "bulk mail" | "垃圾邮件" => FolderRole::Junk,
        "archive" | "archived" | "归档" => FolderRole::Archive,
        "flagged" | "starred" | "星标" => FolderRole::Flagged,
        "[gmail]/all mail" | "all mail" => FolderRole::All,
        _ => FolderRole::Custom,
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().trim_matches('"').replace('\\', "/").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_use_wins() {
        assert_eq!(folder_role(&["sent".into()], "whatever"), FolderRole::Sent);
    }

    #[test]
    fn chinese_sent_name() {
        assert_eq!(folder_role(&[], "已发送"), FolderRole::Sent);
    }
}
