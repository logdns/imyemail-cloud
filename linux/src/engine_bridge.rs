//! core 事件/命令桥。GTK 主循环把调用丢到工作线程。

use chck_engine::CoreEngine;
use chck_types::{
    Account, AccountId, AddAccountRequest, AttachmentHandle, ConnectProbe, DraftId, EngineEvent,
    Flags, Folder, FolderId, MailEngine, MessageBody, MessageId, MessageSummary, Page,
    ProviderInfo, SearchQuery, SendRequest,
};

pub struct EngineBridge {
    engine: CoreEngine,
}

#[allow(dead_code)]
impl EngineBridge {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let engine = {
            #[cfg(feature = "secret")]
            {
                CoreEngine::open_with_os_secrets(path, Box::new(crate::secrets::LibsecretBackend))
            }
            #[cfg(not(feature = "secret"))]
            {
                CoreEngine::open(path)
            }
        }
        .map_err(|e| e.to_string())?;
        Ok(Self { engine })
    }

    pub fn providers(&self) -> Result<Vec<ProviderInfo>, String> {
        self.engine.list_providers().map_err(|e| e.to_string())
    }

    pub fn add_account(&self, req: AddAccountRequest) -> Result<Account, String> {
        self.engine.add_account(req).map_err(|e| e.to_string())
    }

    pub fn test_account(&self, req: &AddAccountRequest) -> Result<ConnectProbe, String> {
        self.engine.test_account(req).map_err(|e| e.to_string())
    }

    pub fn accounts(&self) -> Result<Vec<Account>, String> {
        self.engine.list_accounts().map_err(|e| e.to_string())
    }

    pub fn folders(&self, account_id: &AccountId) -> Result<Vec<Folder>, String> {
        self.engine.list_folders(account_id).map_err(|e| e.to_string())
    }

    pub fn sync(&self, folder_id: &FolderId) -> Result<(), String> {
        self.engine.sync_folder(folder_id).map_err(|e| e.to_string())
    }

    pub fn messages(&self, folder_id: &FolderId) -> Result<Vec<MessageSummary>, String> {
        self.engine
            .list_messages(folder_id, Page { offset: 0, limit: 100 })
            .map_err(|e| e.to_string())
    }

    pub fn unified_inbox(&self) -> Result<Vec<MessageSummary>, String> {
        self.engine.unified_inbox(Page { offset: 0, limit: 100 }).map_err(|e| e.to_string())
    }

    pub fn body(&self, message_id: &MessageId) -> Result<MessageBody, String> {
        self.engine.body(message_id).map_err(|e| e.to_string())
    }

    pub fn send(&self, req: SendRequest) -> Result<String, String> {
        self.engine.send(req).map(|id| id.as_str().to_string()).map_err(|e| e.to_string())
    }

    pub fn undo_send(&self, draft_id: &str) -> Result<(), String> {
        self.engine.undo_send(&DraftId::from(draft_id)).map_err(|e| e.to_string())
    }

    pub fn search(&self, text: &str) -> Result<Vec<MessageSummary>, String> {
        let hits = self
            .engine
            .search(&SearchQuery { text: text.into(), ..SearchQuery::default() })
            .map_err(|e| e.to_string())?;
        let inbox = self.unified_inbox()?;
        let ids: std::collections::HashSet<_> =
            hits.iter().map(|h| h.message_id.as_str().to_string()).collect();
        let matched: Vec<_> =
            inbox.iter().filter(|m| ids.contains(m.id.as_str())).cloned().collect();
        if !matched.is_empty() {
            return Ok(matched);
        }
        let q = text.to_ascii_lowercase();
        Ok(inbox
            .into_iter()
            .filter(|m| {
                m.subject.to_ascii_lowercase().contains(&q)
                    || m.snippet.to_ascii_lowercase().contains(&q)
                    || crate::util::from_display(&m.from).to_ascii_lowercase().contains(&q)
            })
            .collect())
    }

    pub fn delete_message(&self, id: &MessageId) -> Result<(), String> {
        self.engine.delete_message(id).map_err(|e| e.to_string())
    }

    pub fn set_flags(&self, id: &MessageId, flags: Flags) -> Result<(), String> {
        self.engine.set_flags(id, flags).map_err(|e| e.to_string())
    }

    pub fn poll_events(&self) -> Vec<EngineEvent> {
        self.engine.poll_events()
    }

    pub fn attachments(&self, message_id: &MessageId) -> Result<Vec<AttachmentHandle>, String> {
        self.engine.list_attachments(message_id).map_err(|e| e.to_string())
    }

    pub fn flush_due_sends(&self) -> Result<u32, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.engine.flush_due_sends(now).map_err(|e| e.to_string())
    }

    pub fn tick(&self, account_id: Option<&AccountId>, idle: bool) -> Result<(), String> {
        self.engine.tick(account_id, idle, None).map(|_| ()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_include_qq() {
        let dir = std::env::temp_dir().join(format!("linux-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let bridge = EngineBridge::open(dir.join("mail.db")).unwrap();
        let providers = bridge.providers().unwrap();
        assert!(providers.iter().any(|p| p.id == "qq"));
        let acc = bridge.add_account(AddAccountRequest::new("dev@qq.com")).unwrap();
        assert_eq!(acc.provider_id, "qq");
        assert!(bridge.unified_inbox().unwrap().is_empty());
        assert!(bridge.search("hello").unwrap().is_empty());
        assert_eq!(acc.email, "dev@qq.com");
        assert_eq!(bridge.flush_due_sends().unwrap(), 0);
    }
}
