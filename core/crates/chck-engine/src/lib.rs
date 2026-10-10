//! MailEngine：连接探测、文件夹/头同步、tick / IDLE。

use chck_providers::ProviderCatalog;
use chck_store::Store;
use chck_types::{
    Account, AccountId, AccountMode, AccountState, AddAccountRequest, AttachmentHandle,
    AttachmentId, AuthKind, CalendarInvite, ConnectProbe, Contact, DraftId, EngineError,
    EngineEvent, Flags, Folder, FolderId, FolderRole, MailEngine, MessageBody, MessageId,
    MessageSummary, OAuthStart, Page, ProviderInfo, Rsvp, Rule, RuleAction, SearchHit, SearchQuery,
    SendQueueItem, SendRequest, Signature, SyncState, SyncTick, ThreadId, authorize_url, pkce_s256,
    pkce_verifier,
};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

mod secrets;

pub use secrets::{FileSecretStore, OverlaySecretStore, SecretBackend};

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use secrets::KeychainSecretStore;

pub struct CoreEngine {
    store: Store,
    catalog: ProviderCatalog,
    events: Mutex<Vec<EngineEvent>>,
    secrets: Box<dyn SecretBackend>,
    sync: Mutex<chck_sync::SyncEngine>,
}

impl CoreEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref();
        Self::open_with_secrets(path, Box::new(FileSecretStore::open_beside(path)))
    }

    pub fn open_in_memory() -> Result<Self, EngineError> {
        Ok(Self {
            store: Store::open_in_memory()?,
            catalog: ProviderCatalog::builtin(),
            events: Mutex::new(Vec::new()),
            secrets: Box::new(FileSecretStore::memory()),
            sync: Mutex::new(chck_sync::SyncEngine::default()),
        })
    }

    pub fn open_with_secrets(
        path: impl AsRef<Path>,
        secrets: Box<dyn SecretBackend>,
    ) -> Result<Self, EngineError> {
        let path = path.as_ref();
        Ok(Self {
            store: Store::open(path)?,
            catalog: ProviderCatalog::builtin(),
            events: Mutex::new(Vec::new()),
            secrets,
            sync: Mutex::new(chck_sync::SyncEngine::default()),
        })
    }

    pub fn open_with_os_secrets(
        path: impl AsRef<Path>,
        os: Box<dyn SecretBackend>,
    ) -> Result<Self, EngineError> {
        let path = path.as_ref();
        let overlay = OverlaySecretStore::new(os, FileSecretStore::open_beside(path));
        Self::open_with_secrets(path, Box::new(overlay))
    }

    pub fn open_with_strict_os_secrets(
        path: impl AsRef<Path>,
        os: Box<dyn SecretBackend>,
    ) -> Result<Self, EngineError> {
        Self::open_with_secrets(path.as_ref(), os)
    }

    #[must_use]
    pub fn catalog(&self) -> &ProviderCatalog {
        &self.catalog
    }

    fn push(&self, event: EngineEvent) {
        if let Ok(mut q) = self.events.lock() {
            q.push(event);
        }
    }

    /// Public settings never contain credentials, even when an OS secret store is available.
    pub fn account_settings(&self, id: &AccountId) -> Result<serde_json::Value, EngineError> {
        let account = self.store.get_account(id)?.ok_or(EngineError::NotFound)?;
        let raw =
            self.store.get_kv(&format!("imap:{}", id.as_str()))?.ok_or(EngineError::NotFound)?;
        let mut value: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| EngineError::Storage(e.to_string()))?;
        for key in ["password", "api_token", "access_token", "refresh_token"] {
            value.as_object_mut().unwrap().remove(key);
        }
        value["email"] = account.email.into();
        value["display_name"] = account.display_name.into();
        Ok(value)
    }

    pub fn test_account_settings(
        &self,
        id: Option<&AccountId>,
        mut req: AddAccountRequest,
    ) -> Result<ConnectProbe, EngineError> {
        if let Some(id) = id {
            let old = self.load_imap(id)?.ok_or(EngineError::NotFound)?;
            if req.password.as_ref().is_none_or(|p| p.is_empty()) {
                req.password = old.password;
            }
            if req.access_token.is_none() {
                req.access_token = old.access_token;
            }
        }
        let mut result = self.test_account(&req)?;
        let provider = self.catalog.by_email(&req.email);
        apply_provider_defaults(&mut req, provider);
        match chck_proto_smtp::test_connect(&req, provider) {
            Ok(()) => result.push_ok(chck_types::ProbeKind::Smtp, "SMTP TLS / authentication OK"),
            Err(error) => result.push_fail(chck_types::ProbeKind::Smtp, error.to_string()),
        }
        Ok(result)
    }

    pub fn retry_failed_sends(&self) -> Result<(), EngineError> {
        self.store.retry_failed_sends()
    }

    pub fn clear_failed_queue(&self) -> Result<(usize, usize), EngineError> {
        self.store.clear_failed_queue()
    }

    pub fn update_credentials(
        &self,
        id: &AccountId,
        req: AddAccountRequest,
    ) -> Result<Account, EngineError> {
        let Some(mut account) = self.store.get_account(id)? else {
            return Err(EngineError::NotFound);
        };
        let mut req = req;
        if req.email.is_empty() {
            req.email = account.email.clone();
        }
        let provider = self.catalog.by_email(&req.email);
        if let Some(name) = req.display_name.clone().filter(|s| !s.is_empty()) {
            account.display_name = name;
        }
        account.email = req.email.clone();
        account.provider_id = provider.map_or(account.provider_id.clone(), |p| p.id.clone());
        self.store.update_account(&account)?;
        if let Some(existing) = self.load_imap(id)? {
            if req.imap_host.is_none() {
                req.imap_host = existing.imap_host;
            }
            if req.imap_port.is_none() {
                req.imap_port = existing.imap_port;
            }
            if req.smtp_host.is_none() {
                req.smtp_host = existing.smtp_host;
            }
            if req.smtp_port.is_none() {
                req.smtp_port = existing.smtp_port;
            }
            if req.username.is_none() {
                req.username = existing.username;
            }
            if req.access_token.as_ref().is_none_or(|s| s.is_empty()) {
                req.access_token = existing.access_token;
            }
            if req.refresh_token.as_ref().is_none_or(|s| s.is_empty()) {
                req.refresh_token = existing.refresh_token;
            }
            if req.api_base.is_none() {
                req.api_base = existing.api_base;
            }
        }
        apply_provider_defaults(&mut req, provider);
        self.save_imap(id, &req)?;
        Ok(account)
    }

    fn save_imap(&self, id: &AccountId, req: &AddAccountRequest) -> Result<(), EngineError> {
        let json = serde_json::json!({
            "email": req.email,
            "imap_host": req.imap_host,
            "imap_port": req.imap_port,
            "imap_starttls": req.imap_starttls,
            "smtp_host": req.smtp_host,
            "smtp_port": req.smtp_port,
            "smtp_starttls": req.smtp_starttls,
            "accept_invalid_certs": req.accept_invalid_certs,
            "username": req.username,
            "api_base": req.api_base,
        });
        self.store.set_kv(&format!("imap:{}", id.as_str()), &json.to_string())?;
        if let Some(password) = req.password.as_ref().filter(|s| !s.is_empty()) {
            self.secrets.set(id.as_str(), password)?;
        }
        if let Some(token) = req.api_token.as_ref().filter(|s| !s.is_empty()) {
            self.secrets.set(&format!("token:{}", id.as_str()), token)?;
        }
        if let Some(token) = req.access_token.as_ref().filter(|s| !s.is_empty()) {
            self.secrets.set(&format!("access:{}", id.as_str()), token)?;
        }
        if let Some(token) = req.refresh_token.as_ref().filter(|s| !s.is_empty()) {
            self.secrets.set(&format!("refresh:{}", id.as_str()), token)?;
        }
        Ok(())
    }

    fn load_imap(&self, id: &AccountId) -> Result<Option<AddAccountRequest>, EngineError> {
        let Some(raw) = self.store.get_kv(&format!("imap:{}", id.as_str()))? else {
            return Ok(None);
        };
        let v: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| EngineError::Storage(e.to_string()))?;
        let mut req = AddAccountRequest::new(v["email"].as_str().unwrap_or_default());
        req.imap_host = v["imap_host"].as_str().filter(|s| !s.is_empty()).map(ToString::to_string);
        req.imap_port = v["imap_port"].as_u64().map(|n| n as u16);
        req.imap_starttls = v["imap_starttls"].as_bool().unwrap_or(false);
        req.smtp_host = v["smtp_host"].as_str().filter(|s| !s.is_empty()).map(ToString::to_string);
        req.smtp_port = v["smtp_port"].as_u64().map(|n| n as u16);
        req.smtp_starttls = v["smtp_starttls"].as_bool().unwrap_or(false);
        req.accept_invalid_certs = v["accept_invalid_certs"].as_bool().unwrap_or(false);
        req.username = v["username"].as_str().map(ToString::to_string);
        req.api_base = v["api_base"].as_str().filter(|s| !s.is_empty()).map(ToString::to_string);
        req.api_token = self.secrets.get(&format!("token:{}", id.as_str()));
        req.access_token = self.secrets.get(&format!("access:{}", id.as_str()));
        req.refresh_token = self.secrets.get(&format!("refresh:{}", id.as_str()));
        let provider = self.catalog.by_email(&req.email);
        apply_provider_defaults(&mut req, provider);
        req.password = self
            .secrets
            .get(id.as_str())
            .or_else(|| v["password"].as_str().filter(|s| !s.is_empty()).map(ToString::to_string));
        Ok(Some(req))
    }

    fn imap_session(
        &self,
        account_id: &AccountId,
        req: &AddAccountRequest,
        provider: Option<&chck_providers::Provider>,
    ) -> Result<chck_proto_imap::ImapSession, EngineError> {
        match chck_proto_imap::ImapClient::login(req, provider) {
            Ok(session) => Ok(session),
            Err(EngineError::AuthFailed)
                if req.refresh_token.as_deref().is_some_and(|s| !s.is_empty()) =>
            {
                let mut refreshed = req.clone();
                self.refresh_oauth(account_id, &mut refreshed)?;
                chck_proto_imap::ImapClient::login(&refreshed, provider)
            }
            Err(err) => Err(err),
        }
    }

    fn smtp_submit(
        &self,
        account_id: &AccountId,
        req: &AddAccountRequest,
        send: &SendRequest,
        provider: Option<&chck_providers::Provider>,
    ) -> Result<(), EngineError> {
        match chck_proto_smtp::submit(req, send, provider) {
            Ok(()) => Ok(()),
            Err(EngineError::AuthFailed)
                if req.refresh_token.as_deref().is_some_and(|s| !s.is_empty()) =>
            {
                let mut refreshed = req.clone();
                self.refresh_oauth(account_id, &mut refreshed)?;
                chck_proto_smtp::submit(&refreshed, send, provider)
            }
            Err(err) => Err(err),
        }
    }

    fn refresh_oauth(
        &self,
        account_id: &AccountId,
        req: &mut AddAccountRequest,
    ) -> Result<(), EngineError> {
        let refresh = req
            .refresh_token
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(EngineError::AuthExpired("OAuth"))?;
        let provider =
            self.catalog.by_email(&req.email).ok_or(EngineError::AuthExpired("OAuth"))?;
        let token_url =
            provider.auth.token_url.as_deref().ok_or(EngineError::AuthExpired("OAuth"))?;
        let client_id = oauth_client_id(provider).ok_or(EngineError::AuthExpired("OAuth"))?;
        let set = chck_imyemail::refresh_access_token(
            &chck_imyemail::TlsHttp,
            token_url,
            &client_id,
            refresh,
        )?;
        req.access_token = Some(set.access_token);
        if let Some(next) = set.refresh_token.filter(|s| !s.is_empty()) {
            req.refresh_token = Some(next);
        }
        self.save_imap(account_id, req)
    }

    fn discover_folders(&self, account_id: &AccountId) -> Result<Vec<Folder>, EngineError> {
        let Some(req) = self.load_imap(account_id)? else {
            return Ok(Vec::new());
        };
        if req.imap_host.is_none() || !req.has_mail_secret() {
            return self.store.list_folders(account_id);
        }
        let provider = self.catalog.by_email(&req.email);
        let mut session = self.imap_session(account_id, &req, provider)?;
        let remotes = session.list_folders()?;
        session.logout();
        self.upsert_remote_folders(account_id, remotes)
    }

    fn sync_headers(&self, folder_id: &FolderId) -> Result<(), EngineError> {
        let folder = self.store.get_folder(folder_id)?.ok_or(EngineError::NotFound)?;
        let Some(req) = self.load_imap(&folder.account_id)? else {
            return Err(EngineError::Invalid("imap".into()));
        };
        let provider = self.catalog.by_email(&req.email);
        let mut session = self.imap_session(&folder.account_id, &req, provider)?;
        let result = self.sync_headers_on(&mut session, &folder);
        session.logout();
        result.map(|_| ())
    }

    /// Read the sanitized SQLite body only, without network access or flag changes.
    /// Missing messages and messages whose body has not been fetched return `None`.
    pub fn cached_body(&self, id: &MessageId) -> Result<Option<MessageBody>, EngineError> {
        self.store.get_body(id)
    }

    fn fetch_body(&self, id: &MessageId) -> Result<MessageBody, EngineError> {
        if let Some(cached) = self.cached_body(id)? {
            return Ok(cached);
        }
        let (account_id, _, uid, folder) =
            self.store.get_message_meta(id)?.ok_or(EngineError::NotFound)?;
        let Some(req) = self.load_imap(&account_id)? else {
            return Err(EngineError::Invalid("imap".into()));
        };
        let provider = self.catalog.by_email(&req.email);
        let mut session = self.imap_session(&account_id, &req, provider)?;
        session.select(&folder.path)?;
        let raw = session.fetch_rfc822(uid)?;
        session.logout();
        let parsed = chck_mime::parse(raw.as_bytes())?;
        let sanitized = if let Some(html) = parsed.html {
            chck_sanitize::sanitize_html(&html)
        } else {
            chck_sanitize::SanitizeResult {
                html: format!("<pre>{}</pre>", html_escape(&parsed.text)),
                remote_blocked: 0,
            }
        };
        let body = MessageBody {
            id: id.clone(),
            html_sanitized: sanitized.html,
            text: parsed.text,
            remote_blocked: sanitized.remote_blocked,
        };
        self.store.put_body(&body)?;
        for (i, att) in parsed.attachments.iter().enumerate() {
            let handle = AttachmentHandle {
                id: AttachmentId::from(format!("{}:{i}", id.as_str())),
                name: att.name.clone(),
                mime: att.mime.clone(),
                size: att.data.len() as u64,
                cid: att.cid.clone(),
            };
            self.store.upsert_attachment(id, &handle, &att.data)?;
        }
        Ok(body)
    }

    fn flush_ops(&self) -> Result<(), EngineError> {
        for (op_id, account_id, op_type, payload) in self.store.list_pending_ops()? {
            // Legacy optimistic moves did not retain UIDVALIDITY or COPYUID.
            // Replaying them could copy/delete a different message after a reset.
            if op_type == "move" {
                self.store.mark_op(op_id, "failed")?;
                continue;
            }
            let Some(req) = self.load_imap(&account_id)? else {
                self.store.mark_op(op_id, "failed")?;
                continue;
            };
            if req.imap_host.is_none() {
                continue;
            }
            let provider = self.catalog.by_email(&req.email);
            let mut session = match self.imap_session(&account_id, &req, provider) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let v: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or_else(|_| serde_json::json!({}));
            let uid = v["uid"].as_u64().unwrap_or(0) as u32;
            let (Some(source), Some(validity)) = (v["source"].as_str(), v["uidvalidity"].as_u64())
            else {
                self.store.mark_op(op_id, "failed")?;
                session.logout();
                continue;
            };
            match session.select(source) {
                Ok(status) if validity != 0 && u64::from(status.uidvalidity) == validity => {}
                Ok(_) => {
                    self.store.mark_op(op_id, "failed")?;
                    session.logout();
                    continue;
                }
                Err(_) => {
                    session.logout();
                    continue;
                }
            }
            let result = match op_type.as_str() {
                "flag" => {
                    let bits = v["flags"].as_i64().unwrap_or(0);
                    let flags = Flags::from_bits(bits);
                    let mut tokens = Vec::new();
                    if flags.seen {
                        tokens.push("\\Seen");
                    }
                    if flags.flagged {
                        tokens.push("\\Flagged");
                    }
                    if flags.deleted {
                        tokens.push("\\Deleted");
                    }
                    session.uid_store_flags(uid, &tokens.join(" "))
                }
                "delete" => session.require_uidplus().and_then(|()| session.uid_delete_only(uid)),
                _ => Ok(()),
            };
            session.logout();
            self.store.mark_op(op_id, if result.is_ok() { "done" } else { "failed" })?;
        }
        Ok(())
    }

    fn flush_send(&self, draft_id: &DraftId) -> Result<(), EngineError> {
        let Some((account_id, json, state)) = self.store.get_outbox(draft_id.as_str())? else {
            return Err(EngineError::NotFound);
        };
        if state == "cancelled" {
            return Ok(());
        }
        self.store.set_outbox_state(draft_id.as_str(), "sending", None)?;
        let v: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| EngineError::Storage(e.to_string()))?;
        let mut send = SendRequest {
            account_id: account_id.clone(),
            to: json_strings(&v["to"]),
            cc: json_strings(&v["cc"]),
            bcc: json_strings(&v["bcc"]),
            subject: v["subject"].as_str().unwrap_or("").to_string(),
            body_text: v["body_text"].as_str().unwrap_or("").to_string(),
            body_html: v["body_html"].as_str().map(ToString::to_string),
            undo_window_secs: 10,
        };
        if let Some(sig) = self.store.default_signature(&account_id)? {
            apply_message_signature(&mut send, &sig.body);
        }
        let Some(req) = self.load_imap(&account_id)? else {
            return Err(EngineError::Invalid("smtp".into()));
        };
        let provider = self.catalog.by_email(&req.email);
        self.smtp_submit(&account_id, &req, &send, provider)?;
        let _ = self.append_sent(&account_id, &req, &send, provider);
        self.store.set_outbox_state(draft_id.as_str(), "sent", None)?;
        self.push(EngineEvent::SendQueueChanged {
            draft_id: draft_id.clone(),
            state: chck_types::OutboxState::Sent,
            error: None,
        });
        Ok(())
    }

    fn append_sent(
        &self,
        account_id: &AccountId,
        req: &AddAccountRequest,
        send: &SendRequest,
        provider: Option<&chck_providers::Provider>,
    ) -> Result<(), EngineError> {
        let folders = self.store.list_folders(&send.account_id)?;
        let Some(sent) = folders.into_iter().find(|f| f.role == FolderRole::Sent) else {
            return Ok(());
        };
        let mut session = self.imap_session(account_id, req, provider)?;
        let rfc = chck_proto_smtp::rfc822(&req.email, send);
        let result = session.append_rfc822(&sent.path, &rfc);
        session.logout();
        result
    }

    fn import_contacts(
        &self,
        account_id: Option<&AccountId>,
        contacts: Vec<Contact>,
    ) -> Result<u32, EngineError> {
        let mut n = 0u32;
        for mut c in contacts {
            if c.email.is_empty() {
                continue;
            }
            c.account_id = account_id.cloned();
            if let Some(existing) = self.store.find_contact_email(&c.email)? {
                c.id = existing.id;
                c.vip |= existing.vip;
                if c.name.is_none() {
                    c.name = existing.name;
                }
            } else if c.id.is_empty() {
                c.id = Uuid::new_v4().to_string();
            }
            self.store.upsert_contact(&c)?;
            n += 1;
        }
        Ok(n)
    }

    fn set_account_state(&self, id: &AccountId, state: AccountState) -> Result<(), EngineError> {
        let Some(mut account) = self.store.get_account(id)? else {
            return Ok(());
        };
        if account.state == state {
            return Ok(());
        }
        account.state = state;
        self.store.update_account(&account)?;
        self.push(EngineEvent::AccountStateChanged { account_id: id.clone(), state });
        Ok(())
    }

    fn upsert_remote_folders(
        &self,
        account_id: &AccountId,
        remotes: Vec<chck_proto_imap::RemoteFolder>,
    ) -> Result<Vec<Folder>, EngineError> {
        for remote in remotes {
            let folder = Folder {
                id: FolderId::from(format!("{}:{}", account_id.as_str(), remote.path)),
                account_id: account_id.clone(),
                remote_name: remote.path.clone(),
                path: remote.path,
                role: remote.role,
                unread_count: 0,
                total_count: 0,
            };
            self.store.upsert_folder(&folder)?;
        }
        self.store.list_folders(account_id)
    }

    fn persist_headers(
        &self,
        folder: &Folder,
        items: Vec<chck_proto_imap::EnvelopeFetch>,
        from_uid: u32,
    ) -> Result<u32, EngineError> {
        let count = items.len() as u32;
        let mut last_uid = from_uid;
        let mut top = Vec::new();
        for item in items {
            let snippet: String = item.headers.subject.chars().take(140).collect();
            let thread_id = self.store.find_thread_id(
                &folder.account_id,
                item.headers.in_reply_to.as_deref(),
                item.headers.message_id.as_deref(),
            );
            let id = self.store.upsert_message(
                folder,
                &chck_store::MessageRecord {
                    uid: item.uid,
                    subject: item.headers.subject.clone(),
                    from: item.headers.from.clone(),
                    to: item.headers.to.clone(),
                    date_unix: item.headers.date_unix,
                    snippet: snippet.clone(),
                    flags: item.flags,
                    has_attachments: item.headers.has_attachments,
                    message_id: item.headers.message_id.clone(),
                    in_reply_to: item.headers.in_reply_to.clone(),
                    thread_id,
                    size: item.size,
                },
            )?;
            last_uid = last_uid.max(item.uid.saturating_add(1));
            let _ = self.apply_rules(&id);
            if top.len() < 3 {
                top.push(MessageSummary {
                    id,
                    account_id: folder.account_id.clone(),
                    folder_id: folder.id.clone(),
                    thread_id: item.headers.message_id.clone().map(ThreadId::from),
                    subject: item.headers.subject,
                    from: item.headers.from,
                    to: item.headers.to,
                    date_unix: item.headers.date_unix,
                    snippet,
                    flags: item.flags,
                    has_attachments: item.headers.has_attachments,
                    labels: Vec::new(),
                });
            }
        }
        let (total, unread) = self.store.message_count(&folder.id)?;
        self.store.set_folder_cursor(&folder.id, last_uid, total, unread)?;
        self.push(EngineEvent::SyncProgress {
            account_id: folder.account_id.clone(),
            folder_id: folder.id.clone(),
            done: count,
            total: count,
        });
        if count > 0 {
            self.push(EngineEvent::NewMessages {
                account_id: folder.account_id.clone(),
                folder_id: folder.id.clone(),
                count,
                top,
            });
        }
        Ok(count)
    }

    fn sync_headers_on(
        &self,
        session: &mut chck_proto_imap::ImapSession,
        folder: &Folder,
    ) -> Result<u32, EngineError> {
        self.push(EngineEvent::SyncProgress {
            account_id: folder.account_id.clone(),
            folder_id: folder.id.clone(),
            done: 0,
            total: 0,
        });
        let status = session.select(&folder.path)?;
        self.store.set_folder_uidvalidity(&folder.id, status.uidvalidity)?;
        let from_uid = self.store.folder_cursor(&folder.id)?;
        let items = session.fetch_headers_since(from_uid, 200)?;
        self.persist_headers(folder, items, from_uid)
    }

    fn tick_account(
        &self,
        account: &Account,
        idle: bool,
        idle_wait: Duration,
    ) -> Result<SyncTick, EngineError> {
        let mut machine = self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        machine.bind(account.id.clone());
        let now = Instant::now();
        machine.tick(now)?;
        if machine.state == SyncState::Backoff {
            let state = machine.state;
            drop(machine);
            return Ok(SyncTick {
                account_id: Some(account.id.clone()),
                state,
                folder_id: None,
                supports_idle: false,
            });
        }
        drop(machine);

        let Some(req) = self.load_imap(&account.id)? else {
            return Ok(SyncTick {
                account_id: Some(account.id.clone()),
                state: SyncState::Idle,
                folder_id: None,
                supports_idle: false,
            });
        };
        if req.imap_host.is_none() || !req.has_mail_secret() {
            return Ok(SyncTick {
                account_id: Some(account.id.clone()),
                state: SyncState::Idle,
                folder_id: None,
                supports_idle: false,
            });
        }

        self.set_account_state(&account.id, AccountState::Syncing)?;
        let _ = self.flush_ops();
        let provider = self.catalog.by_email(&req.email);
        let mut session = match self.imap_session(&account.id, &req, provider) {
            Ok(s) => s,
            Err(err) => {
                let mut machine =
                    self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                machine.fail(Instant::now());
                let state = machine.state;
                drop(machine);
                let acc_state =
                    if matches!(err, EngineError::AuthFailed | EngineError::AuthExpired(_)) {
                        AccountState::AuthRequired
                    } else {
                        AccountState::Error
                    };
                self.set_account_state(&account.id, acc_state)?;
                return Ok(SyncTick {
                    account_id: Some(account.id.clone()),
                    state,
                    folder_id: None,
                    supports_idle: false,
                });
            }
        };

        let mut machine = self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        machine.connected();
        drop(machine);

        let remotes = match session.list_folders() {
            Ok(r) => r,
            Err(err) => {
                session.logout();
                let mut machine =
                    self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                machine.fail(Instant::now());
                drop(machine);
                self.set_account_state(&account.id, AccountState::Error)?;
                return Err(err);
            }
        };
        let folders = self.upsert_remote_folders(&account.id, remotes)?;
        let supports_idle = session.supports_idle().unwrap_or(false);
        let folder_ids: Vec<FolderId> = folders.iter().map(|f| f.id.clone()).collect();
        let mut machine = self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        machine.discovered(folder_ids, supports_idle);
        drop(machine);

        let mut last_folder = None;
        loop {
            let next = {
                let mut machine =
                    self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                machine.next_folder()
            };
            let Some(folder_id) = next else { break };
            last_folder = Some(folder_id.clone());
            if let Some(folder) = folders.iter().find(|f| f.id == folder_id)
                && let Err(err) = self.sync_headers_on(&mut session, folder)
            {
                session.logout();
                let mut machine =
                    self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                machine.fail(Instant::now());
                drop(machine);
                self.set_account_state(&account.id, AccountState::Error)?;
                return Err(err);
            }
        }

        let state;
        if idle {
            let inbox = folders.iter().find(|f| f.role == FolderRole::Inbox).or(folders.first());
            if let Some(inbox) = inbox {
                last_folder = Some(inbox.id.clone());
                if let Err(err) = session.select(&inbox.path) {
                    session.logout();
                    return Err(err);
                }
                let wake = if supports_idle {
                    match session.idle_wait(idle_wait) {
                        Ok(chck_proto_imap::IdleWake::Unsupported) => noop_wake(&mut session),
                        other => other,
                    }
                } else {
                    noop_wake(&mut session)
                };
                match wake {
                    Ok(
                        chck_proto_imap::IdleWake::Exists(_) | chck_proto_imap::IdleWake::Expunge,
                    ) => {
                        let mut machine =
                            self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                        machine.idle_event();
                        drop(machine);
                        let _ = self.sync_headers_on(&mut session, inbox);
                        state = SyncState::Done;
                    }
                    Ok(_) => {
                        let mut machine =
                            self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                        machine.finish(true);
                        state = machine.state;
                    }
                    Err(_) => {
                        let mut machine =
                            self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                        machine.fail(Instant::now());
                        state = machine.state;
                    }
                }
            } else {
                let mut machine =
                    self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
                machine.finish(true);
                state = machine.state;
            }
        } else {
            let mut machine = self.sync.lock().map_err(|_| EngineError::Storage("lock".into()))?;
            machine.finish(false);
            state = machine.state;
        }
        session.logout();
        if matches!(state, SyncState::Backoff) {
            self.set_account_state(&account.id, AccountState::Error)?;
        } else {
            self.set_account_state(&account.id, AccountState::Online)?;
        }
        Ok(SyncTick {
            account_id: Some(account.id.clone()),
            state,
            folder_id: last_folder,
            supports_idle,
        })
    }
}

fn noop_wake(
    session: &mut chck_proto_imap::ImapSession,
) -> Result<chck_proto_imap::IdleWake, EngineError> {
    session.noop().map(|res| {
        res.untagged
            .iter()
            .map(|l| chck_proto_imap::parse_idle_event(l))
            .find(|w| {
                matches!(
                    w,
                    chck_proto_imap::IdleWake::Exists(_) | chck_proto_imap::IdleWake::Expunge
                )
            })
            .unwrap_or(chck_proto_imap::IdleWake::Timeout)
    })
}

fn draft_json(req: &SendRequest) -> serde_json::Value {
    serde_json::json!({
        "to": req.to,
        "cc": req.cc,
        "bcc": req.bcc,
        "subject": req.subject,
        "body_text": req.body_text,
        "body_html": req.body_html,
    })
}

fn json_strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(ToString::to_string)).collect())
        .unwrap_or_default()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn apply_message_signature(send: &mut SendRequest, signature: &str) {
    let text = apply_signature(&send.body_text, signature);
    if text != send.body_text {
        if let Some(html) = &mut send.body_html {
            html.push_str(&format!(
                "<div style=\"white-space:pre-wrap;margin-top:1em\">{}</div>",
                html_escape(signature)
            ));
        }
        send.body_text = text;
    }
}

fn apply_signature(body: &str, sig: &str) -> String {
    if sig.is_empty() || body.contains(sig) {
        body.to_string()
    } else if body.is_empty() {
        sig.to_string()
    } else {
        format!("{body}\n\n{sig}")
    }
}

fn apply_provider_defaults(
    req: &mut AddAccountRequest,
    provider: Option<&chck_providers::Provider>,
) {
    let Some(p) = provider else { return };
    if req.imap_host.as_ref().is_none_or(|h| h.is_empty()) {
        req.imap_host = Some(p.imap.host.clone());
        if req.imap_port.is_none() {
            req.imap_port = Some(p.imap.port);
        }
        if !req.imap_starttls {
            req.imap_starttls = matches!(p.imap.tls, chck_providers::TlsMode::Starttls);
        }
    }
    if req.smtp_host.as_ref().is_none_or(|h| h.is_empty()) {
        req.smtp_host = Some(p.smtp.host.clone());
        if req.smtp_port.is_none() {
            req.smtp_port = Some(p.smtp.port);
        }
        if !req.smtp_starttls {
            req.smtp_starttls = matches!(p.smtp.tls, chck_providers::TlsMode::Starttls);
        }
    }
    if req.auth_kind.is_none() {
        req.auth_kind = Some(match p.auth.kind.as_str() {
            "oauth2" => AuthKind::OAuth2,
            "authcode" => AuthKind::AuthCode,
            "app-password" => AuthKind::AppPassword,
            _ => AuthKind::Password,
        });
    }
}

fn provider_info(p: &chck_providers::Provider) -> ProviderInfo {
    ProviderInfo {
        id: p.id.clone(),
        domains: p.domains.clone(),
        display_name_zh: p.display_name.zh.clone(),
        display_name_en: p.display_name.en.clone(),
        imap_host: p.imap.host.clone(),
        imap_port: p.imap.port,
        smtp_host: p.smtp.host.clone(),
        smtp_port: p.smtp.port,
        smtp_starttls: matches!(p.smtp.tls, chck_providers::TlsMode::Starttls),
        auth_kind: p.auth.kind.clone(),
        help_url: p.auth.help_url.clone(),
        quirks: p.quirks.clone(),
        token_url: p.auth.token_url.clone(),
        auth_url: p.auth.auth_url.clone(),
        client_id_env: p.auth.client_id_env.clone(),
        scopes: p.auth.scopes.clone(),
        redirect_uri: p.auth.redirect_uri.clone(),
    }
}

fn oauth_client_id(provider: &chck_providers::Provider) -> Option<String> {
    let key = provider.auth.client_id_env.as_deref()?;
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

impl MailEngine for CoreEngine {
    fn add_account(&self, mut req: AddAccountRequest) -> Result<Account, EngineError> {
        if !req.email.contains('@') {
            return Err(EngineError::Invalid("email".into()));
        }
        let provider = self.catalog.by_email(&req.email);
        apply_provider_defaults(&mut req, provider);
        let provider_id = provider.map_or("custom", |p| p.id.as_str()).to_string();
        let display_name = req
            .display_name
            .clone()
            .unwrap_or_else(|| req.email.split('@').next().unwrap_or(&req.email).to_string());
        let account = Account {
            id: AccountId::from(Uuid::new_v4().to_string()),
            email: req.email.clone(),
            display_name,
            provider_id,
            mode: if req.api_token.is_some() {
                AccountMode::Enhanced
            } else {
                AccountMode::Standard
            },
            color: "#3D6BFE".into(),
            sync_scope: req.sync_scope,
            state: AccountState::Offline,
        };
        self.store.insert_account(&account)?;
        self.save_imap(&account.id, &req)?;
        self.push(EngineEvent::AccountStateChanged {
            account_id: account.id.clone(),
            state: account.state,
        });
        Ok(account)
    }

    fn update_account(&self, account: Account) -> Result<Account, EngineError> {
        self.store.update_account(&account)?;
        Ok(account)
    }

    fn remove_account(&self, id: &AccountId) -> Result<(), EngineError> {
        self.store.delete_kv(&format!("imap:{}", id.as_str()))?;
        self.secrets.delete(id.as_str())?;
        self.secrets.delete(&format!("token:{}", id.as_str()))?;
        self.secrets.delete(&format!("access:{}", id.as_str()))?;
        self.secrets.delete(&format!("refresh:{}", id.as_str()))?;
        self.store.delete_account(id)
    }

    fn test_account(&self, req: &AddAccountRequest) -> Result<ConnectProbe, EngineError> {
        if !req.email.contains('@') {
            return Err(EngineError::Invalid("email".into()));
        }
        let mut req = req.clone();
        let provider = self.catalog.by_email(&req.email);
        apply_provider_defaults(&mut req, provider);
        chck_proto_imap::ImapClient::test_connect(&req, provider)
    }

    fn list_accounts(&self) -> Result<Vec<Account>, EngineError> {
        self.store.list_accounts()
    }

    fn list_providers(&self) -> Result<Vec<ProviderInfo>, EngineError> {
        Ok(self.catalog.providers.iter().map(provider_info).collect())
    }

    fn list_folders(&self, account_id: &AccountId) -> Result<Vec<Folder>, EngineError> {
        let existing = self.store.list_folders(account_id)?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        self.discover_folders(account_id)
    }

    fn sync_folder(&self, folder_id: &FolderId) -> Result<(), EngineError> {
        self.sync_headers(folder_id)
    }

    fn list_messages(
        &self,
        folder_id: &FolderId,
        page: Page,
    ) -> Result<Vec<MessageSummary>, EngineError> {
        self.store.list_messages(folder_id, page)
    }

    fn unified_inbox(&self, page: Page) -> Result<Vec<MessageSummary>, EngineError> {
        self.store.unified_inbox(page)
    }

    fn list_thread(&self, thread_id: &ThreadId) -> Result<Vec<MessageSummary>, EngineError> {
        self.store.list_thread(thread_id)
    }

    fn body(&self, id: &MessageId) -> Result<MessageBody, EngineError> {
        self.fetch_body(id)
    }

    fn set_flags(&self, id: &MessageId, flags: Flags) -> Result<(), EngineError> {
        self.store.set_message_flags(id, flags)?;
        if let Some((account_id, source_id, uid, source)) = self.store.get_message_meta(id)? {
            let validity = self.store.folder_uidvalidity(&source_id)?;
            self.store.enqueue_op(
                &account_id,
                "flag",
                &serde_json::json!({"id": id.as_str(), "uid": uid, "source": source.path, "uidvalidity": validity, "flags": flags.to_bits()})
                    .to_string(),
            )?;
            let _ = self.flush_ops();
        }
        Ok(())
    }

    fn move_message(&self, id: &MessageId, dest: &FolderId) -> Result<(), EngineError> {
        let dest_folder = self.store.get_folder(dest)?.ok_or(EngineError::NotFound)?;
        let (account_id, source_id, uid, source) =
            self.store.get_message_meta(id)?.ok_or(EngineError::NotFound)?;
        if account_id != dest_folder.account_id {
            return Err(EngineError::Invalid("cross-account move".into()));
        }
        if source_id == *dest {
            return Ok(());
        }
        let req = self.load_imap(&account_id)?.ok_or(EngineError::offline())?;
        let mut session =
            self.imap_session(&account_id, &req, self.catalog.by_email(&req.email))?;
        let result = (|| {
            let selected = session.select(&source.path)?;
            if selected.uidvalidity == 0
                || self.store.folder_uidvalidity(&source.id)? != Some(selected.uidvalidity)
            {
                return Err(EngineError::Invalid(
                    "source UIDVALIDITY changed; synchronize before archiving".into(),
                ));
            }
            if !self.store.claim_move(id, dest)? {
                return Err(EngineError::Invalid(
                    "previous archive is incomplete; reconcile folders before retrying".into(),
                ));
            }
            let (destination_validity, destination_uid) =
                match session.uid_copy_mapped(uid, &dest_folder.path) {
                    Ok(uid) => uid,
                    Err(error @ EngineError::Unsupported(_)) => {
                        self.store.delete_kv(&format!("move:{}", id.as_str()))?;
                        return Err(error);
                    }
                    Err(error) => return Err(error),
                };
            if session.select(&dest_folder.path)?.uidvalidity != destination_validity {
                return Err(EngineError::Server(
                    "destination UIDVALIDITY changed; source retained",
                ));
            }
            self.store.set_folder_uidvalidity(dest, destination_validity)?;
            if session.select(&source.path)?.uidvalidity != selected.uidvalidity {
                return Err(EngineError::Server("source UIDVALIDITY changed; archive incomplete"));
            }
            session.uid_delete_only(uid)?;
            self.store.move_message(id, &dest_folder, destination_uid)?;
            self.store.delete_kv(&format!("move:{}", id.as_str()))
        })();
        session.logout();
        result
    }

    fn delete_message(&self, id: &MessageId) -> Result<(), EngineError> {
        if let Some((account_id, source_id, uid, source)) = self.store.get_message_meta(id)? {
            let validity = self.store.folder_uidvalidity(&source_id)?;
            self.store.delete_message(id)?;
            self.store.enqueue_op(
                &account_id,
                "delete",
                &serde_json::json!({"id": id.as_str(), "uid": uid, "source": source.path, "uidvalidity": validity})
                    .to_string(),
            )?;
            let _ = self.flush_ops();
        }
        Ok(())
    }

    fn bulk_set_flags(&self, ids: &[MessageId], flags: Flags) -> Result<(), EngineError> {
        for id in ids {
            self.set_flags(id, flags)?;
        }
        Ok(())
    }

    fn bulk_delete(&self, ids: &[MessageId]) -> Result<(), EngineError> {
        for id in ids {
            self.delete_message(id)?;
        }
        Ok(())
    }

    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, EngineError> {
        let local = self.store.search(query)?;
        let Some(account_id) = query.account_id.as_ref() else {
            return Ok(local);
        };
        let Some(req) = self.load_imap(account_id)? else {
            return Ok(local);
        };
        let (Some(base), Some(token)) = (req.api_base.as_deref(), req.api_token.as_deref()) else {
            return Ok(local);
        };
        match chck_imyemail::server_search(&chck_imyemail::TlsHttp, base, token, &query.text) {
            Ok(server) => Ok(chck_imyemail::merge_hits(local, server)),
            Err(_) => Ok(local),
        }
    }

    fn probe_enhanced(&self, account_id: &AccountId) -> Result<bool, EngineError> {
        let Some(req) = self.load_imap(account_id)? else {
            return Ok(false);
        };
        let (Some(base), Some(token)) = (req.api_base.as_deref(), req.api_token.as_deref()) else {
            return Ok(false);
        };
        match chck_imyemail::probe(&chck_imyemail::TlsHttp, base, token) {
            Ok(cap) => Ok(cap.product.eq_ignore_ascii_case("imyemail")),
            Err(_) => Ok(false),
        }
    }

    fn schedule_send(&self, req: SendRequest, send_at_unix: i64) -> Result<DraftId, EngineError> {
        let id = DraftId::from(Uuid::new_v4().to_string());
        let json = serde_json::to_string(&draft_json(&req)).unwrap_or_else(|_| "{}".into());
        self.store.insert_outbox_at(
            id.as_str(),
            &req.account_id,
            &json,
            "scheduled",
            Some(send_at_unix),
        )?;
        Ok(id)
    }

    fn flush_due_sends(&self, now_unix: i64) -> Result<u32, EngineError> {
        let ids = self.store.due_outbox(now_unix)?;
        let mut n = 0u32;
        let mut last_err: Option<EngineError> = None;
        for id in ids {
            self.store.set_outbox_state(&id, "queued", None)?;
            match self.flush_send(&DraftId::from(id.clone())) {
                Ok(()) => n += 1,
                Err(err) => {
                    self.store.set_outbox_state(&id, "failed", Some(&err.to_string()))?;
                    self.push(EngineEvent::SendQueueChanged {
                        draft_id: DraftId::from(id),
                        state: chck_types::OutboxState::Failed,
                        error: Some(err.to_string()),
                    });
                    last_err = Some(err);
                }
            }
        }
        if n == 0
            && let Some(err) = last_err
        {
            return Err(err);
        }
        Ok(n)
    }

    fn add_rule(&self, mut rule: Rule) -> Result<Rule, EngineError> {
        if rule.id.is_empty() {
            rule.id = Uuid::new_v4().to_string();
        }
        self.store.upsert_rule(&rule)?;
        Ok(rule)
    }

    fn list_rules(&self, account_id: &AccountId) -> Result<Vec<Rule>, EngineError> {
        self.store.list_rules(account_id)
    }

    fn remove_rule(&self, rule_id: &str) -> Result<(), EngineError> {
        self.store.delete_rule(rule_id)
    }

    fn apply_rules(&self, message_id: &MessageId) -> Result<u32, EngineError> {
        let Some(summary) = self.store.summary(message_id)? else {
            return Ok(0);
        };
        let rules = self.store.list_rules(&summary.account_id)?;
        let from = summary.from.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(" ");
        let to = summary.to.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(" ");
        let mut applied = 0u32;
        for rule in rules.into_iter().filter(|r| r.enabled) {
            if !rule.definition.matches(
                &from,
                &to,
                &summary.subject,
                &summary.snippet,
                summary.has_attachments,
            ) {
                continue;
            }
            applied += 1;
            for action in &rule.definition.actions {
                match action {
                    RuleAction::Flag { seen, flagged } => {
                        let mut flags = summary.flags;
                        if let Some(s) = seen {
                            flags.seen = *s;
                        }
                        if let Some(f) = flagged {
                            flags.flagged = *f;
                        }
                        self.set_flags(message_id, flags)?;
                    }
                    RuleAction::Delete => self.delete_message(message_id)?,
                    RuleAction::Move { folder_path } => {
                        if let Some(dest) =
                            self.store.folder_by_path(&summary.account_id, folder_path)?
                        {
                            self.move_message(message_id, &dest.id)?;
                        }
                    }
                    RuleAction::Label { name } => {
                        self.store.add_label(message_id, name)?;
                    }
                }
            }
        }
        Ok(applied)
    }

    fn snooze(&self, id: &MessageId, until_unix: i64) -> Result<(), EngineError> {
        self.store.set_snooze(id, Some(until_unix))
    }

    fn wake_snoozed(&self, now_unix: i64) -> Result<u32, EngineError> {
        let ids = self.store.due_snoozes(now_unix)?;
        let n = ids.len() as u32;
        for id in ids {
            self.store.set_snooze(&id, None)?;
        }
        Ok(n)
    }

    fn save_signature(&self, mut signature: Signature) -> Result<Signature, EngineError> {
        if signature.id.is_empty() {
            signature.id = Uuid::new_v4().to_string();
        }
        self.store.upsert_signature(&signature)?;
        Ok(signature)
    }

    fn list_signatures(&self, account_id: &AccountId) -> Result<Vec<Signature>, EngineError> {
        self.store.list_signatures(account_id)
    }

    fn remove_signature(&self, signature_id: &str) -> Result<(), EngineError> {
        self.store.delete_signature(signature_id)
    }

    fn list_attachments(
        &self,
        message_id: &MessageId,
    ) -> Result<Vec<AttachmentHandle>, EngineError> {
        let listed = self.store.list_attachments(message_id)?;
        if listed.is_empty() {
            let _ = self.fetch_body(message_id);
            return self.store.list_attachments(message_id);
        }
        Ok(listed)
    }

    fn fetch_attachment(&self, id: &AttachmentId) -> Result<Vec<u8>, EngineError> {
        if let Some(data) = self.store.attachment_data(id)?
            && !data.is_empty()
        {
            return Ok(data);
        }
        let mid = id.as_str().split(':').next().unwrap_or(id.as_str());
        let _ = self.fetch_body(&MessageId::from(mid));
        self.store.attachment_data(id)?.ok_or(EngineError::NotFound)
    }

    fn save_draft(&self, req: SendRequest) -> Result<DraftId, EngineError> {
        let id = DraftId::from(Uuid::new_v4().to_string());
        let json = serde_json::to_string(&draft_json(&req)).unwrap_or_else(|_| "{}".into());
        self.store.insert_outbox(id.as_str(), &req.account_id, &json, "draft")?;
        Ok(id)
    }

    fn send(&self, req: SendRequest) -> Result<DraftId, EngineError> {
        let id = DraftId::from(Uuid::new_v4().to_string());
        let json = serde_json::to_string(&draft_json(&req)).unwrap_or_else(|_| "{}".into());
        if req.undo_window_secs == 0 {
            self.store.insert_outbox(id.as_str(), &req.account_id, &json, "queued")?;
            self.push(EngineEvent::SendQueueChanged {
                draft_id: id.clone(),
                state: chck_types::OutboxState::Queued,
                error: None,
            });
            match self.flush_send(&id) {
                Ok(()) => {}
                Err(err) => {
                    self.store.set_outbox_state(id.as_str(), "failed", Some(&err.to_string()))?;
                    self.push(EngineEvent::SendQueueChanged {
                        draft_id: id.clone(),
                        state: chck_types::OutboxState::Failed,
                        error: Some(err.to_string()),
                    });
                }
            }
            return Ok(id);
        }
        let send_at = unix_now().saturating_add(i64::from(req.undo_window_secs));
        self.store.insert_outbox_at(
            id.as_str(),
            &req.account_id,
            &json,
            "queued",
            Some(send_at),
        )?;
        self.push(EngineEvent::SendQueueChanged {
            draft_id: id.clone(),
            state: chck_types::OutboxState::Queued,
            error: None,
        });
        Ok(id)
    }

    fn undo_send(&self, draft_id: &DraftId) -> Result<(), EngineError> {
        if let Some((_, _, state)) = self.store.get_outbox(draft_id.as_str())?
            && matches!(state.as_str(), "queued" | "scheduled" | "draft")
        {
            self.store.set_outbox_state(draft_id.as_str(), "cancelled", None)?;
            self.push(EngineEvent::SendQueueChanged {
                draft_id: draft_id.clone(),
                state: chck_types::OutboxState::Cancelled,
                error: None,
            });
            return Ok(());
        }
        Err(EngineError::NotFound)
    }

    fn outbox(&self) -> Result<Vec<SendQueueItem>, EngineError> {
        Ok(self
            .store
            .list_outbox()?
            .into_iter()
            .map(|row| SendQueueItem {
                draft_id: DraftId::from(row.id),
                account_id: row.account_id,
                state: match row.state.as_str() {
                    "draft" => chck_types::OutboxState::Draft,
                    "sending" => chck_types::OutboxState::Sending,
                    "sent" => chck_types::OutboxState::Sent,
                    "failed" => chck_types::OutboxState::Failed,
                    "cancelled" => chck_types::OutboxState::Cancelled,
                    _ => chck_types::OutboxState::Queued,
                },
                retry_count: row.retry_count,
                error: row.error,
            })
            .collect())
    }

    fn upsert_contact(&self, mut contact: Contact) -> Result<Contact, EngineError> {
        if contact.id.is_empty() {
            contact.id = Uuid::new_v4().to_string();
        }
        self.store.upsert_contact(&contact)?;
        Ok(contact)
    }

    fn list_contacts(&self, account_id: Option<&AccountId>) -> Result<Vec<Contact>, EngineError> {
        self.store.list_contacts(account_id)
    }

    fn set_vip(&self, contact_id: &str, vip: bool) -> Result<(), EngineError> {
        self.store.set_vip(contact_id, vip)
    }

    fn import_vcard(
        &self,
        account_id: Option<&AccountId>,
        vcard: &str,
    ) -> Result<u32, EngineError> {
        self.import_contacts(account_id, chck_types::parse_vcard(vcard))
    }

    fn import_csv(&self, account_id: Option<&AccountId>, csv: &str) -> Result<u32, EngineError> {
        self.import_contacts(account_id, chck_types::parse_csv(csv))
    }

    fn parse_invite(&self, ics: &str) -> Result<CalendarInvite, EngineError> {
        let inv = chck_types::parse_ics(ics);
        if inv.uid.is_empty() {
            return Err(EngineError::Invalid("ics".into()));
        }
        Ok(inv)
    }

    fn rsvp_invite(&self, invite: &CalendarInvite, rsvp: Rsvp) -> Result<String, EngineError> {
        let attendee = self
            .store
            .list_accounts()?
            .into_iter()
            .next()
            .map(|a| a.email)
            .unwrap_or_else(|| "user@localhost".into());
        Ok(chck_types::rsvp_ics(invite, rsvp, &attendee))
    }

    fn diagnose(&self, req: &AddAccountRequest) -> Result<ConnectProbe, EngineError> {
        self.test_account(req)
    }

    fn export_backup(&self) -> Result<String, EngineError> {
        let dump = serde_json::json!({
            "version": 1,
            "accounts": self.store.list_accounts()?,
            "contacts": self.store.list_contacts(None)?,
            "rules": self.store.list_all_rules()?,
            "signatures": self.store.list_all_signatures()?,
        });
        serde_json::to_string_pretty(&dump).map_err(|e| EngineError::Storage(e.to_string()))
    }

    fn start_oauth(&self, email: &str) -> Result<OAuthStart, EngineError> {
        if !email.contains('@') {
            return Err(EngineError::Invalid("email".into()));
        }
        let provider = self
            .catalog
            .by_email(email)
            .ok_or_else(|| EngineError::Invalid("oauth provider".into()))?;
        if provider.auth.kind != "oauth2" {
            return Err(EngineError::Invalid("oauth provider".into()));
        }
        let auth_url = provider
            .auth
            .auth_url
            .as_deref()
            .ok_or_else(|| EngineError::Invalid("auth url".into()))?;
        let client_id =
            oauth_client_id(provider).ok_or_else(|| EngineError::Invalid("client_id".into()))?;
        let redirect_uri = provider
            .auth
            .redirect_uri
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8743/oauth".into());
        let verifier = pkce_verifier();
        let pkce = pkce_s256(&verifier);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let state = format!("{nanos:x}");
        Ok(OAuthStart {
            authorize_url: authorize_url(
                auth_url,
                &client_id,
                &redirect_uri,
                &provider.auth.scopes,
                &state,
                &pkce.challenge,
            ),
            verifier: pkce.verifier,
            state,
            redirect_uri,
        })
    }

    fn complete_oauth(
        &self,
        email: &str,
        code: &str,
        verifier: &str,
        redirect_uri: Option<&str>,
    ) -> Result<AddAccountRequest, EngineError> {
        if !email.contains('@') {
            return Err(EngineError::Invalid("email".into()));
        }
        let provider = self
            .catalog
            .by_email(email)
            .ok_or_else(|| EngineError::Invalid("oauth provider".into()))?;
        let token_url = provider
            .auth
            .token_url
            .as_deref()
            .ok_or_else(|| EngineError::Invalid("token url".into()))?;
        let client_id =
            oauth_client_id(provider).ok_or_else(|| EngineError::Invalid("client_id".into()))?;
        let redirect = redirect_uri
            .map(ToString::to_string)
            .or_else(|| provider.auth.redirect_uri.clone())
            .unwrap_or_else(|| "http://127.0.0.1:8743/oauth".into());
        let set = chck_imyemail::exchange_code(
            &chck_imyemail::TlsHttp,
            token_url,
            &client_id,
            code,
            &redirect,
            verifier,
        )?;
        let mut req = AddAccountRequest::new(email);
        apply_provider_defaults(&mut req, Some(provider));
        req.auth_kind = Some(AuthKind::OAuth2);
        req.access_token = Some(set.access_token);
        req.refresh_token = set.refresh_token;
        Ok(req)
    }

    fn import_backup(&self, json: &str) -> Result<u32, EngineError> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| EngineError::Invalid(e.to_string()))?;
        let mut n = 0u32;
        if let Some(arr) = v["accounts"].as_array() {
            for item in arr {
                if let Ok(acc) = serde_json::from_value::<Account>(item.clone()) {
                    if self.store.get_account(&acc.id)?.is_none() {
                        self.store.insert_account(&acc)?;
                    }
                    n += 1;
                }
            }
        }
        if let Some(arr) = v["contacts"].as_array() {
            for item in arr {
                if let Ok(c) = serde_json::from_value::<Contact>(item.clone()) {
                    self.store.upsert_contact(&c)?;
                    n += 1;
                }
            }
        }
        if let Some(arr) = v["rules"].as_array() {
            for item in arr {
                if let Ok(r) = serde_json::from_value::<Rule>(item.clone()) {
                    self.store.upsert_rule(&r)?;
                    n += 1;
                }
            }
        }
        if let Some(arr) = v["signatures"].as_array() {
            for item in arr {
                if let Ok(s) = serde_json::from_value::<Signature>(item.clone()) {
                    self.store.upsert_signature(&s)?;
                    n += 1;
                }
            }
        }
        Ok(n)
    }

    fn poll_events(&self) -> Vec<EngineEvent> {
        self.events.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }

    fn tick(
        &self,
        account_id: Option<&AccountId>,
        idle: bool,
        idle_wait_ms: Option<u64>,
    ) -> Result<Vec<SyncTick>, EngineError> {
        let wait = Duration::from_millis(idle_wait_ms.unwrap_or(if idle {
            chck_sync::IDLE_KEEPALIVE.as_millis() as u64
        } else {
            0
        }));
        let accounts = if let Some(id) = account_id {
            vec![self.store.get_account(id)?.ok_or(EngineError::NotFound)?]
        } else {
            self.store.list_accounts()?
        };
        let mut out = Vec::with_capacity(accounts.len());
        for account in accounts {
            out.push(self.tick_account(&account, idle, wait)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_move_queue_is_quarantined_without_network_replay() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let account = engine.add_account(AddAccountRequest::new("legacy@example.test")).unwrap();
        engine
            .store
            .enqueue_op(&account.id, "move", r#"{"id":"legacy:INBOX:1","uid":1,"dest":"Archive"}"#)
            .unwrap();
        assert_eq!(engine.store.list_pending_ops().unwrap().len(), 1);
        engine.flush_ops().unwrap();
        assert!(engine.store.list_pending_ops().unwrap().is_empty());
    }
    use chck_types::MailEngine;

    #[test]
    fn blank_password_update_keeps_saved_secret() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let mut req = AddAccountRequest::new("a@custom.test");
        req.password = Some("original-secret".into());
        req.imap_host = Some("imap.custom.test".into());
        let account = engine.add_account(req).unwrap();
        let mut update = AddAccountRequest::new("a@custom.test");
        update.smtp_host = Some("smtp.custom.test".into());
        update.password = Some(String::new());
        engine.update_credentials(&account.id, update).unwrap();
        assert_eq!(
            engine.load_imap(&account.id).unwrap().unwrap().password.as_deref(),
            Some("original-secret")
        );
    }

    #[test]
    fn manual_retry_recovers_exhausted_failed_outbox_only() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let account = engine.add_account(AddAccountRequest::new("a@custom.test")).unwrap();
        engine.store.insert_outbox("failed", &account.id, "{}", "queued").unwrap();
        engine.store.insert_outbox("sent", &account.id, "{}", "sent").unwrap();
        engine.store.insert_outbox("draft", &account.id, "{}", "draft").unwrap();
        for _ in 0..3 {
            engine.store.set_outbox_state("failed", "failed", Some("smtp")).unwrap();
        }
        assert!(engine.store.due_outbox(1).unwrap().is_empty());
        engine.retry_failed_sends().unwrap();
        assert_eq!(engine.store.due_outbox(1).unwrap(), vec!["failed"]);
    }

    #[test]
    fn add_qq_account_resolves_provider() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("dev@qq.com")).unwrap();
        assert_eq!(acc.provider_id, "qq");
        assert_eq!(acc.mode, AccountMode::Standard);
        assert_eq!(engine.list_accounts().unwrap().len(), 1);
        let events = engine.poll_events();
        assert!(matches!(events[0], EngineEvent::AccountStateChanged { .. }));
    }

    #[test]
    fn update_credentials_keeps_id_and_renames() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("dev@qq.com")).unwrap();
        let mut req = AddAccountRequest::new("dev@qq.com");
        req.display_name = Some("Alice".into());
        req.password = Some("secret".into());
        let updated = engine.update_credentials(&acc.id, req).unwrap();
        assert_eq!(updated.id, acc.id);
        assert_eq!(updated.display_name, "Alice");
        assert_eq!(engine.list_accounts().unwrap().len(), 1);
    }

    #[test]
    fn custom_domain_is_custom_provider() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("me@example.com")).unwrap();
        assert_eq!(acc.provider_id, "custom");
    }

    #[test]
    fn test_account_custom_without_host_fails_dns() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let probe = engine.test_account(&AddAccountRequest::new("me@example.com")).unwrap();
        assert!(!probe.success());
        let dns = probe.step(chck_types::ProbeKind::Dns).unwrap();
        assert_eq!(dns.status, chck_types::ProbeStatus::Failed);
    }

    #[test]
    fn rules_match_and_flag_on_apply() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        engine
            .add_rule(Rule {
                id: String::new(),
                account_id: acc.id.clone(),
                name: "invoices".into(),
                enabled: true,
                definition: chck_types::RuleDef {
                    match_all: false,
                    conditions: vec![chck_types::RuleCondition::Subject {
                        contains: "invoice".into(),
                    }],
                    actions: vec![chck_types::RuleAction::Flag {
                        seen: Some(true),
                        flagged: Some(true),
                    }],
                },
            })
            .unwrap();
        assert_eq!(engine.list_rules(&acc.id).unwrap().len(), 1);
        let folder = Folder {
            id: FolderId::from(format!("{}:INBOX", acc.id.as_str())),
            account_id: acc.id.clone(),
            remote_name: "INBOX".into(),
            path: "INBOX".into(),
            role: chck_types::FolderRole::Inbox,
            unread_count: 0,
            total_count: 0,
        };
        engine.store.upsert_folder(&folder).unwrap();
        let mid = engine
            .store
            .upsert_message(
                &folder,
                &chck_store::MessageRecord {
                    uid: 9,
                    subject: "Invoice #1".into(),
                    from: vec![chck_types::Address { name: None, email: "bill@ex.com".into() }],
                    to: vec![chck_types::Address { name: None, email: "a@gmail.com".into() }],
                    date_unix: 1,
                    snippet: "pay".into(),
                    flags: Flags::default(),
                    has_attachments: false,
                    message_id: Some("<inv@ex.com>".into()),
                    in_reply_to: None,
                    thread_id: None,
                    size: 1,
                },
            )
            .unwrap();
        assert_eq!(engine.apply_rules(&mid).unwrap(), 1);
        let row = engine.store.summary(&mid).unwrap().unwrap();
        assert!(row.flags.seen && row.flags.flagged);

        engine
            .add_rule(Rule {
                id: String::new(),
                account_id: acc.id.clone(),
                name: "to-me".into(),
                enabled: true,
                definition: chck_types::RuleDef {
                    match_all: false,
                    conditions: vec![chck_types::RuleCondition::To {
                        contains: "a@gmail.com".into(),
                    }],
                    actions: vec![chck_types::RuleAction::Label { name: "work".into() }],
                },
            })
            .unwrap();
        assert!(engine.apply_rules(&mid).unwrap() >= 1);
        let labeled = engine.store.summary(&mid).unwrap().unwrap();
        assert!(labeled.labels.iter().any(|l| l == "work"));
        assert_eq!(labeled.to[0].email, "a@gmail.com");
    }

    #[test]
    fn snooze_then_wake() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        let folder = Folder {
            id: FolderId::from(format!("{}:INBOX", acc.id.as_str())),
            account_id: acc.id.clone(),
            remote_name: "INBOX".into(),
            path: "INBOX".into(),
            role: chck_types::FolderRole::Inbox,
            unread_count: 0,
            total_count: 0,
        };
        engine.store.upsert_folder(&folder).unwrap();
        let mid = engine
            .store
            .upsert_message(
                &folder,
                &chck_store::MessageRecord {
                    uid: 1,
                    subject: "later".into(),
                    from: vec![],
                    to: vec![],
                    date_unix: 1,
                    snippet: String::new(),
                    flags: Flags::default(),
                    has_attachments: false,
                    message_id: None,
                    in_reply_to: None,
                    thread_id: None,
                    size: 0,
                },
            )
            .unwrap();
        engine.snooze(&mid, unix_now().saturating_add(3_600)).unwrap();
        assert!(engine.list_messages(&folder.id, Page::default()).unwrap().is_empty());
        assert!(engine.unified_inbox(Page::default()).unwrap().is_empty());
        engine.snooze(&mid, 100).unwrap();
        assert_eq!(engine.wake_snoozed(50).unwrap(), 0);
        assert_eq!(engine.wake_snoozed(100).unwrap(), 1);
        assert_eq!(engine.wake_snoozed(100).unwrap(), 0);
        assert_eq!(engine.list_messages(&folder.id, Page::default()).unwrap().len(), 1);
    }

    #[test]
    fn schedule_send_stays_until_due() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        let id = engine
            .schedule_send(
                SendRequest {
                    account_id: acc.id,
                    to: vec!["b@ex.com".into()],
                    subject: "later".into(),
                    ..SendRequest::default()
                },
                9_999_999_999,
            )
            .unwrap();
        assert_eq!(engine.flush_due_sends(1).unwrap(), 0);
        let box_ = engine.outbox().unwrap();
        assert!(box_.iter().any(|i| i.draft_id == id));
    }

    #[test]
    fn enhanced_without_api_is_not_probed() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        assert!(!engine.probe_enhanced(&acc.id).unwrap());
    }

    #[test]
    fn unified_inbox_empty_without_messages() {
        let engine = CoreEngine::open_in_memory().unwrap();
        engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        assert!(engine.unified_inbox(Page::default()).unwrap().is_empty());
    }

    #[test]
    fn save_draft_and_undo() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        let id = engine
            .save_draft(SendRequest {
                account_id: acc.id,
                to: vec!["b@ex.com".into()],
                subject: "Hi".into(),
                body_text: "x".into(),
                ..SendRequest::default()
            })
            .unwrap();
        engine.undo_send(&id).unwrap();
        assert_eq!(engine.outbox().unwrap()[0].state, chck_types::OutboxState::Cancelled);
    }

    #[test]
    fn send_undo_window_stays_queued() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        let id = engine
            .send(SendRequest {
                account_id: acc.id,
                to: vec!["b@ex.com".into()],
                subject: "Hi".into(),
                undo_window_secs: 30,
                ..SendRequest::default()
            })
            .unwrap();
        assert_eq!(engine.outbox().unwrap()[0].state, chck_types::OutboxState::Queued);
        assert_eq!(engine.flush_due_sends(1).unwrap(), 0);
        engine.undo_send(&id).unwrap();
        assert_eq!(engine.outbox().unwrap()[0].state, chck_types::OutboxState::Cancelled);
    }

    #[test]
    fn html_signature_is_escaped_and_added_once() {
        let mut send = SendRequest {
            body_text: "Hello".into(),
            body_html: Some("<p>Hello</p>".into()),
            ..SendRequest::default()
        };
        apply_message_signature(&mut send, "Dev <dev@example.test>\n<script>no</script>");
        assert!(send.body_text.ends_with("<script>no</script>"));
        let html = send.body_html.clone().unwrap();
        assert!(html.contains("&lt;dev@example.test&gt;"));
        assert!(!html.contains("<script>"));
        apply_message_signature(&mut send, "Dev <dev@example.test>\n<script>no</script>");
        assert_eq!(send.body_html.unwrap(), html);
    }

    #[test]
    fn signature_roundtrip_and_apply() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        engine
            .save_signature(Signature {
                id: String::new(),
                account_id: acc.id.clone(),
                name: "work".into(),
                body: "--\nDev".into(),
                is_default: true,
            })
            .unwrap();
        assert_eq!(engine.list_signatures(&acc.id).unwrap().len(), 1);
        assert_eq!(apply_signature("hello", "--\nDev"), "hello\n\n--\nDev");
        assert_eq!(apply_signature("hello\n\n--\nDev", "--\nDev"), "hello\n\n--\nDev");
    }

    #[test]
    fn flag_without_message_is_noop() {
        let engine = CoreEngine::open_in_memory().unwrap();
        engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        engine.set_flags(&MessageId::from("m1"), Flags::default()).unwrap();
        assert_eq!(engine.store.pending_ops_count().unwrap(), 0);
    }

    #[test]
    fn add_qq_fills_imap_from_catalog() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("dev@qq.com")).unwrap();
        let req = engine.load_imap(&acc.id).unwrap().unwrap();
        assert_eq!(req.imap_host.as_deref(), Some("imap.qq.com"));
        assert_eq!(req.imap_port, Some(993));
        assert_eq!(req.smtp_host.as_deref(), Some("smtp.qq.com"));
        assert!(!req.smtp_starttls);
        assert_eq!(req.auth_kind, Some(AuthKind::AuthCode));
        let providers = engine.list_providers().unwrap();
        assert_eq!(providers.len(), 20);
        assert!(providers.iter().any(|p| p.id == "qq" && p.imap_host == "imap.qq.com"));
        assert!(providers.iter().any(|p| {
            p.id == "gmail" && p.token_url.as_deref() == Some("https://oauth2.googleapis.com/token")
        }));
    }

    #[test]
    fn password_stays_out_of_sqlite() {
        let dir = std::env::temp_dir().join(format!("chck-secret-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let engine = CoreEngine::open(&db).unwrap();
        let mut req = AddAccountRequest::new("a@gmail.com");
        req.password = Some("secret-pass".into());
        let acc = engine.add_account(req).unwrap();
        let raw = engine.store.get_kv(&format!("imap:{}", acc.id.as_str())).unwrap().unwrap();
        assert!(!raw.contains("secret-pass"), "{raw}");
        let loaded = engine.load_imap(&acc.id).unwrap().unwrap();
        assert_eq!(loaded.password.as_deref(), Some("secret-pass"));
        let sidecar = std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap();
        assert!(sidecar.contains("secret-pass"));
        engine.remove_account(&acc.id).unwrap();
        let gone = std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap();
        assert!(!gone.contains("secret-pass"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oauth_tokens_stay_out_of_sqlite() {
        let dir = std::env::temp_dir().join(format!("chck-oauth-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let engine = CoreEngine::open(&db).unwrap();
        let mut req = AddAccountRequest::new("a@gmail.com");
        req.access_token = Some("ya29.secret".into());
        req.refresh_token = Some("1//refresh".into());
        let acc = engine.add_account(req).unwrap();
        let raw = engine.store.get_kv(&format!("imap:{}", acc.id.as_str())).unwrap().unwrap();
        assert!(!raw.contains("ya29.secret"), "{raw}");
        assert!(!raw.contains("1//refresh"), "{raw}");
        let loaded = engine.load_imap(&acc.id).unwrap().unwrap();
        assert_eq!(loaded.access_token.as_deref(), Some("ya29.secret"));
        assert_eq!(loaded.refresh_token.as_deref(), Some("1//refresh"));
        assert!(loaded.uses_xoauth2());
        let sidecar = std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap();
        assert!(sidecar.contains("ya29.secret"));
        engine.remove_account(&acc.id).unwrap();
        let gone = std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap();
        assert!(!gone.contains("ya29.secret"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_oauth_rejects_non_oauth_provider() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let err = engine.start_oauth("dev@qq.com").unwrap_err();
        assert!(matches!(err, EngineError::Invalid(_)));
        match engine.start_oauth("a@gmail.com") {
            Ok(start) => {
                assert!(start.authorize_url.contains("accounts.google.com"));
                assert!(start.authorize_url.contains("code_challenge"));
                assert!(!start.verifier.is_empty());
            }
            Err(EngineError::Invalid(_)) => {}
            Err(other) => panic!("{other}"),
        }
    }

    #[test]
    fn os_secret_backend_roundtrip() {
        use std::collections::HashMap;
        use std::sync::Mutex;

        struct MapBackend(Mutex<HashMap<String, String>>);
        impl SecretBackend for MapBackend {
            fn get(&self, id: &str) -> Option<String> {
                self.0.lock().ok()?.get(id).cloned()
            }
            fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
                self.0
                    .lock()
                    .map_err(|_| EngineError::Storage("lock".into()))?
                    .insert(id.into(), secret.into());
                Ok(())
            }
            fn delete(&self, id: &str) -> Result<(), EngineError> {
                self.0.lock().map_err(|_| EngineError::Storage("lock".into()))?.remove(id);
                Ok(())
            }
        }

        let dir = std::env::temp_dir().join(format!("chck-os-secret-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let engine =
            CoreEngine::open_with_os_secrets(&db, Box::new(MapBackend(Mutex::new(HashMap::new()))))
                .unwrap();
        let mut req = AddAccountRequest::new("a@gmail.com");
        req.password = Some("os-pass".into());
        let acc = engine.add_account(req).unwrap();
        assert_eq!(
            engine.load_imap(&acc.id).unwrap().unwrap().password.as_deref(),
            Some("os-pass")
        );
        let sidecar =
            std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap_or_default();
        assert!(!sidecar.contains("os-pass"), "{sidecar}");
        engine.remove_account(&acc.id).unwrap();
        assert!(engine.load_imap(&acc.id).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn apple_os_secrets_skip_sidecar() {
        let dir = std::env::temp_dir().join(format!("chck-kc-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let engine =
            CoreEngine::open_with_os_secrets(&db, Box::new(KeychainSecretStore::new())).unwrap();
        let mut req = AddAccountRequest::new("a@gmail.com");
        req.password = Some("kc-engine-pass".into());
        let acc = engine.add_account(req).unwrap();
        assert_eq!(
            engine.load_imap(&acc.id).unwrap().unwrap().password.as_deref(),
            Some("kc-engine-pass")
        );
        let sidecar =
            std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap_or_default();
        assert!(!sidecar.contains("kc-engine-pass"), "{sidecar}");
        engine.remove_account(&acc.id).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn outlook_smtp_is_starttls_587() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("me@outlook.com")).unwrap();
        let req = engine.load_imap(&acc.id).unwrap().unwrap();
        assert_eq!(req.smtp_host.as_deref(), Some("smtp.office365.com"));
        assert_eq!(req.smtp_port, Some(587));
        assert!(req.smtp_starttls);
        let providers = engine.list_providers().unwrap();
        assert!(providers.iter().any(|p| p.id == "outlook" && p.smtp_starttls));
    }

    #[test]
    fn attachments_roundtrip() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        let folder = Folder {
            id: FolderId::from(format!("{}:INBOX", acc.id.as_str())),
            account_id: acc.id.clone(),
            remote_name: "INBOX".into(),
            path: "INBOX".into(),
            role: FolderRole::Inbox,
            unread_count: 0,
            total_count: 0,
        };
        engine.store.upsert_folder(&folder).unwrap();
        let mid = engine
            .store
            .upsert_message(
                &folder,
                &chck_store::MessageRecord {
                    uid: 1,
                    subject: "file".into(),
                    from: vec![],
                    to: vec![],
                    date_unix: 1,
                    snippet: String::new(),
                    flags: Flags::default(),
                    has_attachments: true,
                    message_id: None,
                    in_reply_to: None,
                    thread_id: None,
                    size: 0,
                },
            )
            .unwrap();
        let handle = AttachmentHandle {
            id: AttachmentId::from(format!("{}:0", mid.as_str())),
            name: "a.pdf".into(),
            mime: "application/pdf".into(),
            size: 3,
            cid: None,
        };
        engine.store.upsert_attachment(&mid, &handle, &[1, 2, 3]).unwrap();
        assert_eq!(engine.list_attachments(&mid).unwrap()[0].name, "a.pdf");
        assert_eq!(engine.fetch_attachment(&handle.id).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn contacts_vcard_csv_and_vip() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        assert_eq!(
            engine
                .import_vcard(
                    Some(&acc.id),
                    "BEGIN:VCARD\nFN:Alice\nEMAIL:alice@ex.com\nEND:VCARD\n"
                )
                .unwrap(),
            1
        );
        assert_eq!(engine.import_csv(Some(&acc.id), "email,name\nbob@ex.com,Bob\n").unwrap(), 1);
        let list = engine.list_contacts(Some(&acc.id)).unwrap();
        assert_eq!(list.len(), 2);
        engine.set_vip(&list[0].id, true).unwrap();
        assert!(engine.list_contacts(Some(&acc.id)).unwrap().iter().any(|c| c.vip));
    }

    #[test]
    fn ics_parse_and_rsvp() {
        let engine = CoreEngine::open_in_memory().unwrap();
        engine.add_account(AddAccountRequest::new("me@ex.com")).unwrap();
        let inv = engine
            .parse_invite(
                "BEGIN:VEVENT\nUID:x1\nSUMMARY:Sync\nORGANIZER:MAILTO:a@ex.com\nEND:VEVENT\n",
            )
            .unwrap();
        assert_eq!(inv.uid, "x1");
        let reply = engine.rsvp_invite(&inv, Rsvp::Decline).unwrap();
        assert!(reply.contains("DECLINED"));
        assert!(reply.contains("UID:x1"));
    }

    #[test]
    fn backup_roundtrip_without_secrets() {
        let engine = CoreEngine::open_in_memory().unwrap();
        let acc = engine.add_account(AddAccountRequest::new("a@gmail.com")).unwrap();
        engine
            .upsert_contact(Contact {
                id: "c1".into(),
                account_id: Some(acc.id.clone()),
                email: "p@ex.com".into(),
                name: Some("Pat".into()),
                vip: true,
            })
            .unwrap();
        let json = engine.export_backup().unwrap();
        assert!(!json.contains("password"));
        let other = CoreEngine::open_in_memory().unwrap();
        assert!(other.import_backup(&json).unwrap() >= 2);
        assert_eq!(other.list_contacts(None).unwrap()[0].email, "p@ex.com");
        assert!(!other.diagnose(&AddAccountRequest::new("a@gmail.com")).unwrap().success());
    }
}
