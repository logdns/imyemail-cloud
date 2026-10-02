//! JSON FFI 门面。各端（含 uniFFI 生成层）只走字符串契约，不接触 IMAP/SQL。

use chck_engine::{CoreEngine, SecretBackend};
use chck_types::{
    AccountId, AddAccountRequest, AttachmentId, DraftId, Flags, FolderId, MailEngine, MessageId,
    Page, SendRequest, ThreadId,
};
use serde_json::json;

pub struct FfiEngine {
    inner: CoreEngine,
}

impl FfiEngine {
    pub fn open(path: &str) -> Result<Self, String> {
        if path == ":memory:" {
            return Self::open_with_file_secrets(path);
        }
        if std::env::var("IMYEMAIL_CLOUD_SECRETS").ok().as_deref() == Some("file") {
            return Self::open_with_file_secrets(path);
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            CoreEngine::open_with_os_secrets(
                path,
                Box::new(chck_engine::KeychainSecretStore::new()),
            )
            .map(|inner| Self { inner })
            .map_err(err)
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            Self::open_with_file_secrets(path)
        }
    }

    pub fn open_with_file_secrets(path: &str) -> Result<Self, String> {
        CoreEngine::open(path).map(|inner| Self { inner }).map_err(err)
    }

    pub fn open_with_os_secrets(path: &str, os: Box<dyn SecretBackend>) -> Result<Self, String> {
        CoreEngine::open_with_os_secrets(path, os).map(|inner| Self { inner }).map_err(err)
    }

    pub fn add_account(&self, req_json: &str) -> Result<String, String> {
        let req: AddAccountRequest = serde_json::from_str(req_json).map_err(json_err)?;
        to_json(self.inner.add_account(req).map_err(err)?)
    }

    pub fn test_account(&self, req_json: &str) -> Result<String, String> {
        let req: AddAccountRequest = serde_json::from_str(req_json).map_err(json_err)?;
        to_json(self.inner.test_account(&req).map_err(err)?)
    }

    pub fn list_accounts(&self) -> Result<String, String> {
        to_json(self.inner.list_accounts().map_err(err)?)
    }

    pub fn list_providers(&self) -> Result<String, String> {
        to_json(self.inner.list_providers().map_err(err)?)
    }

    pub fn update_account(&self, acc_json: &str) -> Result<String, String> {
        if let Ok(req) = serde_json::from_str::<AddAccountRequest>(acc_json)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(acc_json)
            && let Some(id) = value.get("id").and_then(|v| v.as_str())
        {
            let _ = req;
            let mut merged = self.inner.account_settings(&AccountId::from(id)).map_err(err)?;
            merged
                .as_object_mut()
                .ok_or("invalid settings")?
                .extend(value.as_object().ok_or("invalid settings")?.clone());
            let req = serde_json::from_value(merged).map_err(json_err)?;
            return to_json(self.inner.update_credentials(&AccountId::from(id), req).map_err(err)?);
        }
        let acc: chck_types::Account = serde_json::from_str(acc_json).map_err(json_err)?;
        to_json(self.inner.update_account(acc).map_err(err)?)
    }

    pub fn remove_account(&self, id: &str) -> Result<(), String> {
        self.inner.remove_account(&AccountId::from(id)).map_err(err)
    }

    pub fn list_folders(&self, account_id: &str) -> Result<String, String> {
        to_json(self.inner.list_folders(&AccountId::from(account_id)).map_err(err)?)
    }

    pub fn sync_folder(&self, folder_id: &str) -> Result<(), String> {
        self.inner.sync_folder(&FolderId::from(folder_id)).map_err(err)
    }

    pub fn unified_inbox(&self, offset: u32, limit: u32) -> Result<String, String> {
        to_json(self.inner.unified_inbox(Page { offset, limit }).map_err(err)?)
    }

    pub fn list_thread(&self, thread_id: &str) -> Result<String, String> {
        to_json(self.inner.list_thread(&ThreadId::from(thread_id)).map_err(err)?)
    }

    pub fn delete_message(&self, message_id: &str) -> Result<(), String> {
        self.inner.delete_message(&MessageId::from(message_id)).map_err(err)
    }

    pub fn move_message(&self, message_id: &str, dest_folder_id: &str) -> Result<(), String> {
        self.inner
            .move_message(&MessageId::from(message_id), &FolderId::from(dest_folder_id))
            .map_err(err)
    }

    pub fn bulk_set_flags(&self, ids_json: &str, flags_json: &str) -> Result<(), String> {
        let ids: Vec<String> = serde_json::from_str(ids_json).map_err(json_err)?;
        let ids: Vec<MessageId> = ids.into_iter().map(MessageId::from).collect();
        let flags: Flags = serde_json::from_str(flags_json).map_err(json_err)?;
        self.inner.bulk_set_flags(&ids, flags).map_err(err)
    }

    pub fn bulk_delete(&self, ids_json: &str) -> Result<(), String> {
        let ids: Vec<String> = serde_json::from_str(ids_json).map_err(json_err)?;
        let ids: Vec<MessageId> = ids.into_iter().map(MessageId::from).collect();
        self.inner.bulk_delete(&ids).map_err(err)
    }

    pub fn probe_enhanced(&self, account_id: &str) -> Result<bool, String> {
        self.inner.probe_enhanced(&AccountId::from(account_id)).map_err(err)
    }

    pub fn list_messages(
        &self,
        folder_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<String, String> {
        to_json(
            self.inner
                .list_messages(&FolderId::from(folder_id), Page { offset, limit })
                .map_err(err)?,
        )
    }

    pub fn body(&self, message_id: &str) -> Result<String, String> {
        to_json(self.inner.body(&MessageId::from(message_id)).map_err(err)?)
    }

    /// Cache-only body lookup. A cache miss is the JSON value `null`, not an error.
    pub fn cached_body(&self, message_id: &str) -> Result<String, String> {
        to_json(self.inner.cached_body(&MessageId::from(message_id)).map_err(err)?)
    }

    pub fn search(&self, query: &str) -> Result<String, String> {
        let q =
            chck_types::SearchQuery { text: query.into(), ..chck_types::SearchQuery::default() };
        to_json(self.inner.search(&q).map_err(err)?)
    }

    pub fn set_flags(&self, message_id: &str, flags_json: &str) -> Result<(), String> {
        let flags: Flags = serde_json::from_str(flags_json).map_err(json_err)?;
        self.inner.set_flags(&MessageId::from(message_id), flags).map_err(err)
    }

    pub fn save_draft(&self, req_json: &str) -> Result<String, String> {
        let req: SendRequest = serde_json::from_str(req_json).map_err(json_err)?;
        let id = self.inner.save_draft(req).map_err(err)?;
        Ok(json!({ "id": id.as_str() }).to_string())
    }

    pub fn send(&self, req_json: &str) -> Result<String, String> {
        let req: SendRequest = serde_json::from_str(req_json).map_err(json_err)?;
        let id = self.inner.send(req).map_err(err)?;
        Ok(json!({ "id": id.as_str() }).to_string())
    }

    pub fn schedule_send(&self, req_json: &str, send_at_unix: i64) -> Result<String, String> {
        let req: SendRequest = serde_json::from_str(req_json).map_err(json_err)?;
        let id = self.inner.schedule_send(req, send_at_unix).map_err(err)?;
        Ok(json!({ "id": id.as_str() }).to_string())
    }

    pub fn flush_due_sends(&self, now_unix: i64) -> Result<u32, String> {
        self.inner.flush_due_sends(now_unix).map_err(err)
    }

    pub fn add_rule(&self, rule_json: &str) -> Result<String, String> {
        let rule: chck_types::Rule = serde_json::from_str(rule_json).map_err(json_err)?;
        to_json(self.inner.add_rule(rule).map_err(err)?)
    }

    pub fn list_rules(&self, account_id: &str) -> Result<String, String> {
        to_json(self.inner.list_rules(&AccountId::from(account_id)).map_err(err)?)
    }

    pub fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        self.inner.remove_rule(rule_id).map_err(err)
    }

    pub fn apply_rules(&self, message_id: &str) -> Result<u32, String> {
        self.inner.apply_rules(&MessageId::from(message_id)).map_err(err)
    }

    pub fn snooze(&self, message_id: &str, until_unix: i64) -> Result<(), String> {
        self.inner.snooze(&MessageId::from(message_id), until_unix).map_err(err)
    }

    pub fn wake_snoozed(&self, now_unix: i64) -> Result<u32, String> {
        self.inner.wake_snoozed(now_unix).map_err(err)
    }

    pub fn undo_send(&self, draft_id: &str) -> Result<(), String> {
        self.inner.undo_send(&DraftId::from(draft_id)).map_err(err)
    }

    pub fn outbox(&self) -> Result<String, String> {
        to_json(self.inner.outbox().map_err(err)?)
    }

    pub fn save_signature(&self, sig_json: &str) -> Result<String, String> {
        let sig: chck_types::Signature = serde_json::from_str(sig_json).map_err(json_err)?;
        to_json(self.inner.save_signature(sig).map_err(err)?)
    }

    pub fn list_signatures(&self, account_id: &str) -> Result<String, String> {
        to_json(self.inner.list_signatures(&AccountId::from(account_id)).map_err(err)?)
    }

    pub fn remove_signature(&self, signature_id: &str) -> Result<(), String> {
        self.inner.remove_signature(signature_id).map_err(err)
    }

    pub fn list_attachments(&self, message_id: &str) -> Result<String, String> {
        to_json(self.inner.list_attachments(&MessageId::from(message_id)).map_err(err)?)
    }

    pub fn fetch_attachment(&self, id: &str) -> Result<String, String> {
        let data = self.inner.fetch_attachment(&AttachmentId::from(id)).map_err(err)?;
        Ok(json!({ "id": id, "size": data.len(), "data": data }).to_string())
    }

    pub fn contacts(&self, account_id: Option<&str>) -> Result<String, String> {
        let id = account_id.map(AccountId::from);
        to_json(self.inner.list_contacts(id.as_ref()).map_err(err)?)
    }

    pub fn contact_add(&self, json: &str) -> Result<String, String> {
        let c: chck_types::Contact = serde_json::from_str(json).map_err(json_err)?;
        to_json(self.inner.upsert_contact(c).map_err(err)?)
    }

    pub fn contact_vip(&self, id: &str, vip: bool) -> Result<(), String> {
        self.inner.set_vip(id, vip).map_err(err)
    }

    pub fn import_vcard(&self, account_id: Option<&str>, vcard: &str) -> Result<u32, String> {
        let id = account_id.map(AccountId::from);
        self.inner.import_vcard(id.as_ref(), vcard).map_err(err)
    }

    pub fn import_csv(&self, account_id: Option<&str>, csv: &str) -> Result<u32, String> {
        let id = account_id.map(AccountId::from);
        self.inner.import_csv(id.as_ref(), csv).map_err(err)
    }

    pub fn parse_invite(&self, ics: &str) -> Result<String, String> {
        to_json(self.inner.parse_invite(ics).map_err(err)?)
    }

    pub fn rsvp(&self, ics: &str, partstat: &str) -> Result<String, String> {
        let inv = self.inner.parse_invite(ics).map_err(err)?;
        let rsvp = match partstat {
            "decline" => chck_types::Rsvp::Decline,
            "tentative" => chck_types::Rsvp::Tentative,
            _ => chck_types::Rsvp::Accept,
        };
        self.inner.rsvp_invite(&inv, rsvp).map_err(err)
    }

    pub fn diagnose(&self, req_json: &str) -> Result<String, String> {
        let req: AddAccountRequest = serde_json::from_str(req_json).map_err(json_err)?;
        to_json(self.inner.diagnose(&req).map_err(err)?)
    }

    pub fn export_backup(&self) -> Result<String, String> {
        self.inner.export_backup().map_err(err)
    }

    pub fn import_backup(&self, json: &str) -> Result<u32, String> {
        self.inner.import_backup(json).map_err(err)
    }

    pub fn poll_events(&self) -> String {
        serde_json::to_string(&self.inner.poll_events()).unwrap_or_else(|_| "[]".into())
    }

    pub fn start_oauth(&self, email: &str) -> Result<String, String> {
        to_json(self.inner.start_oauth(email).map_err(err)?)
    }

    pub fn complete_oauth(
        &self,
        email: &str,
        code: &str,
        verifier: &str,
        redirect_uri: Option<&str>,
    ) -> Result<String, String> {
        to_json(self.inner.complete_oauth(email, code, verifier, redirect_uri).map_err(err)?)
    }

    pub fn tick(
        &self,
        account_id: Option<&str>,
        idle: bool,
        idle_wait_ms: Option<u64>,
    ) -> Result<String, String> {
        let id = account_id.map(AccountId::from);
        to_json(self.inner.tick(id.as_ref(), idle, idle_wait_ms).map_err(err)?)
    }

    pub fn call(&self, method: &str, args_json: &str) -> Result<String, String> {
        dispatch(self, method, args_json)
    }

    pub fn clear_failed_queue(&self) -> Result<String, String> {
        let (ops, outbox) = self.inner.clear_failed_queue().map_err(err)?;
        to_json(json!({"ops_cleared": ops, "outbox_cleared": outbox, "total": ops + outbox}))
    }
}

fn dispatch(engine: &FfiEngine, method: &str, args_json: &str) -> Result<String, String> {
    let args: serde_json::Value = if args_json.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(args_json).map_err(json_err)?
    };
    let s = |key: &str| -> Result<String, String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("missing {key}"))
    };
    let opt_s = |key: &str| args.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let u32_field = |key: &str, default: u32| {
        args.get(key).and_then(serde_json::Value::as_u64).map(|n| n as u32).unwrap_or(default)
    };
    let i64_field = |key: &str| -> Result<i64, String> {
        args.get(key).and_then(serde_json::Value::as_i64).ok_or_else(|| format!("missing {key}"))
    };
    let bool_field =
        |key: &str| args.get(key).and_then(serde_json::Value::as_bool).unwrap_or(false);
    match method {
        "account_settings" => {
            to_json(engine.inner.account_settings(&AccountId::from(s("account_id")?)).map_err(err)?)
        }
        "retry_failed_sends" => {
            engine.inner.retry_failed_sends().map_err(err)?;
            Ok("{}".into())
        }
        "clear_failed_queue" => engine.clear_failed_queue(),
        "test_account_settings" => {
            let patch: serde_json::Value = serde_json::from_str(&s("json")?).map_err(json_err)?;
            let id = patch["id"].as_str().map(AccountId::from);
            let mut value = if let Some(id) = &id {
                engine.inner.account_settings(id).map_err(err)?
            } else {
                json!({})
            };
            value
                .as_object_mut()
                .ok_or("invalid settings")?
                .extend(patch.as_object().ok_or("invalid settings")?.clone());
            let req = serde_json::from_value(value).map_err(json_err)?;
            to_json(engine.inner.test_account_settings(id.as_ref(), req).map_err(err)?)
        }
        "add_account" => engine.add_account(&s("json")?),
        "update_account" => engine.update_account(&s("json")?),
        "test_account" | "diagnose" => engine.test_account(&s("json")?),
        "list_accounts" => engine.list_accounts(),
        "list_providers" => engine.list_providers(),
        "remove_account" => engine.remove_account(&s("id")?).map(|()| r#"{"ok":true}"#.into()),
        "list_folders" => engine.list_folders(&s("account_id")?),
        "sync_folder" => engine.sync_folder(&s("folder_id")?).map(|()| r#"{"ok":true}"#.into()),
        "unified_inbox" => engine.unified_inbox(u32_field("offset", 0), u32_field("limit", 50)),
        "list_thread" => engine.list_thread(&s("thread_id")?),
        "list_messages" => {
            engine.list_messages(&s("folder_id")?, u32_field("offset", 0), u32_field("limit", 50))
        }
        "body" => engine.body(&s("message_id")?),
        "cached_body" => engine.cached_body(&s("message_id")?),
        "search" => engine.search(&s("query")?),
        "set_flags" => {
            engine.set_flags(&s("message_id")?, &s("flags")?).map(|()| r#"{"ok":true}"#.into())
        }
        "delete_message" => {
            engine.delete_message(&s("message_id")?).map(|()| r#"{"ok":true}"#.into())
        }
        "move_message" => engine
            .move_message(&s("message_id")?, &s("dest_folder_id")?)
            .map(|()| r#"{"ok":true}"#.into()),
        "bulk_delete" => engine.bulk_delete(&s("ids")?).map(|()| r#"{"ok":true}"#.into()),
        "bulk_set_flags" => {
            engine.bulk_set_flags(&s("ids")?, &s("flags")?).map(|()| r#"{"ok":true}"#.into())
        }
        "probe_enhanced" => {
            let ok = engine.probe_enhanced(&s("account_id")?)?;
            Ok(json!({ "ok": ok }).to_string())
        }
        "save_draft" => engine.save_draft(&s("json")?),
        "send" => engine.send(&s("json")?),
        "schedule_send" => engine.schedule_send(&s("json")?, i64_field("send_at_unix")?),
        "flush_due_sends" => {
            let n = engine.flush_due_sends(i64_field("now_unix").unwrap_or_else(|_| unix_now()))?;
            Ok(json!({ "flushed": n }).to_string())
        }
        "undo_send" => engine.undo_send(&s("draft_id")?).map(|()| r#"{"ok":true}"#.into()),
        "outbox" => engine.outbox(),
        "add_rule" => engine.add_rule(&s("json")?),
        "list_rules" => engine.list_rules(&s("account_id")?),
        "remove_rule" => engine.remove_rule(&s("id")?).map(|()| r#"{"ok":true}"#.into()),
        "apply_rules" => {
            let n = engine.apply_rules(&s("message_id")?)?;
            Ok(json!({ "applied": n }).to_string())
        }
        "snooze" => engine
            .snooze(&s("message_id")?, i64_field("until_unix")?)
            .map(|()| r#"{"ok":true}"#.into()),
        "wake_snoozed" => {
            let n = engine.wake_snoozed(i64_field("now_unix").unwrap_or_else(|_| unix_now()))?;
            Ok(json!({ "woke": n }).to_string())
        }
        "save_signature" => engine.save_signature(&s("json")?),
        "list_signatures" => engine.list_signatures(&s("account_id")?),
        "remove_signature" => engine.remove_signature(&s("id")?).map(|()| r#"{"ok":true}"#.into()),
        "list_attachments" => engine.list_attachments(&s("message_id")?),
        "fetch_attachment" => engine.fetch_attachment(&s("id")?),
        "contacts" => engine.contacts(opt_s("account_id").as_deref()),
        "contact_add" => engine.contact_add(&s("json")?),
        "contact_vip" => {
            engine.contact_vip(&s("id")?, bool_field("vip")).map(|()| r#"{"ok":true}"#.into())
        }
        "import_vcard" => {
            let n = engine.import_vcard(opt_s("account_id").as_deref(), &s("data")?)?;
            Ok(json!({ "imported": n }).to_string())
        }
        "import_csv" => {
            let n = engine.import_csv(opt_s("account_id").as_deref(), &s("data")?)?;
            Ok(json!({ "imported": n }).to_string())
        }
        "parse_invite" => engine.parse_invite(&s("data")?),
        "rsvp" => engine.rsvp(&s("data")?, &opt_s("partstat").unwrap_or_else(|| "accept".into())),
        "export_backup" => engine.export_backup(),
        "import_backup" => {
            let n = engine.import_backup(&s("data")?)?;
            Ok(json!({ "imported": n }).to_string())
        }
        "poll_events" => Ok(engine.poll_events()),
        "start_oauth" => engine.start_oauth(&s("email")?),
        "complete_oauth" => engine.complete_oauth(
            &s("email")?,
            &s("code")?,
            &s("verifier")?,
            opt_s("redirect_uri").as_deref(),
        ),
        "tick" => engine.tick(
            opt_s("account_id").as_deref(),
            bool_field("idle"),
            args.get("idle_wait_ms").and_then(serde_json::Value::as_u64),
        ),
        other => Err(format!("unknown method: {other}")),
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(e: impl ToString) -> String {
    e.to_string()
}

fn json_err(e: serde_json::Error) -> String {
    format!("invalid json: {e}")
}

fn to_json<T: serde::Serialize>(value: T) -> Result<String, String> {
    serde_json::to_string(&value).map_err(json_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_account_roundtrip_json() {
        let dir = std::env::temp_dir().join(format!("chck-ffi-{}", uuid_like()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let ffi = FfiEngine::open(db.to_str().unwrap()).unwrap();
        let acc = ffi.add_account(r#"{"email":"dev@qq.com"}"#).unwrap();
        assert!(acc.contains("qq"));
        let list = ffi.list_accounts().unwrap();
        assert!(list.contains("dev@qq.com"));
        let providers = ffi.list_providers().unwrap();
        assert!(providers.contains("imap.qq.com"));
        let via_call = ffi.call("list_accounts", "{}").unwrap();
        assert!(via_call.contains("dev@qq.com"));
        let providers_call = ffi.call("list_providers", "").unwrap();
        assert!(providers_call.contains("imap.qq.com"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn apple_open_stores_password_in_keychain_not_sidecar() {
        let dir = std::env::temp_dir().join(format!("chck-ffi-kc-{}", uuid_like()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let ffi = FfiEngine::open(db.to_str().unwrap()).unwrap();
        let acc = ffi.add_account(r#"{"email":"a@gmail.com","password":"kc-pass"}"#).unwrap();
        let sidecar =
            std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap_or_default();
        assert!(!sidecar.contains("kc-pass"), "{sidecar}");
        let v: serde_json::Value = serde_json::from_str(&acc).unwrap();
        let id = v["id"].as_str().unwrap();
        ffi.remove_account(id).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn uuid_like() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
            as u64
    }
}
