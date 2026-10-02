//! `imyemail-cloud` CLI。Linux 第一客户端：crate 直连 core。

use chck_ffi::FfiEngine;
use chck_types::{AddAccountRequest, SendRequest};
use std::path::{Path, PathBuf};

pub fn default_db() -> PathBuf {
    if let Ok(p) = std::env::var("IMYEMAIL_CLOUD_DB") {
        return PathBuf::from(p);
    }
    dirs_fallback().join("mail.db")
}

fn dirs_fallback() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/imyemail-cloud")
}

pub fn run(args: &[String]) -> Result<String, String> {
    let (db, rest, file_secrets) = parse_db(args);
    if rest.is_empty() {
        return Ok(usage().into());
    }
    if let Some(parent) = db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let cmd = rest[0].as_str();
    if cmd == "help" || cmd == "-h" || cmd == "--help" {
        return Ok(usage().into());
    }
    let path = db.to_str().unwrap();
    let engine = if file_secrets {
        FfiEngine::open_with_file_secrets(path)?
    } else {
        FfiEngine::open(path)?
    };
    match cmd {
        "probe" => cmd_probe(&engine, &rest[1..]),
        "add" => cmd_add(&engine, &rest[1..]),
        "oauth-start" => {
            let email = flag(&rest[1..], "--email").ok_or("missing --email")?;
            engine.start_oauth(&email)
        }
        "oauth-complete" => cmd_oauth_complete(&engine, &rest[1..]),
        "update" => cmd_update(&engine, &rest[1..]),
        "rm-account" => {
            let id = flag(&rest[1..], "--id").ok_or("missing --id")?;
            engine.remove_account(&id)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "accounts" => engine.list_accounts(),
        "providers" => engine.list_providers(),
        "folders" => {
            let id = flag(&rest[1..], "--account").ok_or("missing --account")?;
            engine.list_folders(&id)
        }
        "sync" => {
            let id = flag(&rest[1..], "--folder").ok_or("missing --folder")?;
            engine.sync_folder(&id)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "list" => {
            let id = flag(&rest[1..], "--folder").ok_or("missing --folder")?;
            engine.list_messages(&id, 0, 50)
        }
        "read" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            engine.body(&id)
        }
        "draft" => cmd_draft(&engine, &rest[1..], false),
        "send" => cmd_draft(&engine, &rest[1..], true),
        "flush" => {
            let now: i64 =
                flag(&rest[1..], "--now").and_then(|s| s.parse().ok()).unwrap_or_else(unix_now_cli);
            let n = engine.flush_due_sends(now)?;
            Ok(format!(r#"{{"flushed":{n}}}"#))
        }
        "undo" => {
            let id = flag(&rest[1..], "--draft").ok_or("missing --draft")?;
            engine.undo_send(&id)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "outbox" => engine.outbox(),
        "clear-failed-queue" => engine.clear_failed_queue(),
        "events" => Ok(engine.poll_events()),
        "tick" => {
            let account = flag(&rest[1..], "--account");
            let idle = rest[1..].iter().any(|a| a == "--idle");
            let wait = flag(&rest[1..], "--wait-ms").and_then(|s| s.parse().ok());
            engine.tick(account.as_deref(), idle, wait)
        }
        "ui" => cmd_ui(&engine, &rest[1..]),
        "search" => {
            let q = flag(&rest[1..], "--q").ok_or("missing --q")?;
            engine.search(&q)
        }
        "inbox" => engine.unified_inbox(0, 50),
        "rules" => {
            let id = flag(&rest[1..], "--account").ok_or("missing --account")?;
            engine.list_rules(&id)
        }
        "rule-add" => cmd_rule_add(&engine, &rest[1..]),
        "snooze" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            let until: i64 = flag(&rest[1..], "--until")
                .ok_or("missing --until")?
                .parse()
                .map_err(|_| "bad --until")?;
            engine.snooze(&id, until)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "wake" => {
            let now: i64 =
                flag(&rest[1..], "--now").unwrap_or_else(|| "0".into()).parse().unwrap_or(0);
            let n = engine.wake_snoozed(now)?;
            Ok(format!(r#"{{"woke":{n}}}"#))
        }
        "schedule" => cmd_schedule(&engine, &rest[1..]),
        "signatures" => {
            let id = flag(&rest[1..], "--account").ok_or("missing --account")?;
            engine.list_signatures(&id)
        }
        "sig-add" => cmd_sig_add(&engine, &rest[1..]),
        "attachments" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            engine.list_attachments(&id)
        }
        "attach" => {
            let id = flag(&rest[1..], "--id").ok_or("missing --id")?;
            engine.fetch_attachment(&id)
        }
        "thread" => {
            let id = flag(&rest[1..], "--thread").ok_or("missing --thread")?;
            engine.list_thread(&id)
        }
        "flags" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            let json = flag(&rest[1..], "--json").ok_or("missing --json")?;
            engine.set_flags(&id, &json)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "delete" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            engine.delete_message(&id)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "move" => {
            let id = flag(&rest[1..], "--message").ok_or("missing --message")?;
            let dest = flag(&rest[1..], "--folder").ok_or("missing --folder")?;
            engine.move_message(&id, &dest)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "contacts" => engine.contacts(flag(&rest[1..], "--account").as_deref()),
        "contact-add" => engine.contact_add(&flag(&rest[1..], "--json").ok_or("missing --json")?),
        "vip" => {
            let id = flag(&rest[1..], "--id").ok_or("missing --id")?;
            engine.contact_vip(&id, true)?;
            Ok(r#"{"ok":true}"#.into())
        }
        "vcard" => {
            let n = engine.import_vcard(
                flag(&rest[1..], "--account").as_deref(),
                &flag(&rest[1..], "--data").ok_or("missing --data")?,
            )?;
            Ok(format!(r#"{{"imported":{n}}}"#))
        }
        "csv" => {
            let n = engine.import_csv(
                flag(&rest[1..], "--account").as_deref(),
                &flag(&rest[1..], "--data").ok_or("missing --data")?,
            )?;
            Ok(format!(r#"{{"imported":{n}}}"#))
        }
        "ics" => engine.parse_invite(&flag(&rest[1..], "--data").ok_or("missing --data")?),
        "rsvp" => engine.rsvp(
            &flag(&rest[1..], "--data").ok_or("missing --data")?,
            &flag(&rest[1..], "--partstat").unwrap_or_else(|| "accept".into()),
        ),
        "diagnose" => cmd_probe(&engine, &rest[1..]),
        "backup" => engine.export_backup(),
        "restore" => {
            let n = engine.import_backup(&flag(&rest[1..], "--data").ok_or("missing --data")?)?;
            Ok(format!(r#"{{"imported":{n}}}"#))
        }
        "ffi" => {
            let method = flag(&rest[1..], "--method").ok_or("missing --method")?;
            let args = if has_flag(&rest[1..], "--stdin") {
                use std::io::Read;
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
                input
            } else {
                flag(&rest[1..], "--args").unwrap_or_else(|| "{}".into())
            };
            engine.call(&method, &args)
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn cmd_probe(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let req = account_from_flags(args)?;
    engine.test_account(&serde_json::to_string(&req).map_err(|e| e.to_string())?)
}

fn cmd_add(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let req = account_from_flags(args)?;
    engine.add_account(&serde_json::to_string(&req).map_err(|e| e.to_string())?)
}

fn cmd_oauth_complete(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let email = flag(args, "--email").ok_or("missing --email")?;
    let code = flag(args, "--code").ok_or("missing --code")?;
    let verifier = flag(args, "--verifier").ok_or("missing --verifier")?;
    let redirect = flag(args, "--redirect");
    let req_json = engine.complete_oauth(&email, &code, &verifier, redirect.as_deref())?;
    engine.add_account(&req_json)
}

fn cmd_update(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let id = flag(args, "--id").ok_or("missing --id")?;
    let mut req = AddAccountRequest::new(flag(args, "--email").unwrap_or_default());
    req.display_name = flag(args, "--name");
    req.password = flag(args, "--password");
    req.access_token = flag(args, "--access-token");
    req.refresh_token = flag(args, "--refresh-token");
    req.imap_host = flag(args, "--host");
    req.imap_port = flag(args, "--port").and_then(|s| s.parse().ok());
    req.accept_invalid_certs = has_flag(args, "--insecure");
    let mut value = serde_json::to_value(&req).map_err(|e| e.to_string())?;
    value["id"] = serde_json::Value::String(id);
    engine.update_account(&value.to_string())
}

fn cmd_draft(engine: &FfiEngine, args: &[String], send: bool) -> Result<String, String> {
    let account = flag(args, "--account").ok_or("missing --account")?;
    let to = flag(args, "--to").ok_or("missing --to")?;
    let subject = flag(args, "--subject").unwrap_or_default();
    let body = flag(args, "--body").unwrap_or_default();
    let undo: u32 = flag(args, "--undo").and_then(|s| s.parse().ok()).unwrap_or(0);
    let cc = flag(args, "--cc").unwrap_or_default();
    let bcc = flag(args, "--bcc").unwrap_or_default();
    let req = SendRequest {
        account_id: account.into(),
        to: split_addrs(&to),
        cc: split_addrs(&cc),
        bcc: split_addrs(&bcc),
        subject,
        body_text: body,
        undo_window_secs: if send { undo } else { 10 },
        ..SendRequest::default()
    };
    let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    if send { engine.send(&json) } else { engine.save_draft(&json) }
}

fn cmd_sig_add(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let account = flag(args, "--account").ok_or("missing --account")?;
    let name = flag(args, "--name").unwrap_or_else(|| "default".into());
    let body = flag(args, "--body").unwrap_or_default();
    let json = serde_json::json!({
        "id": "",
        "account_id": account,
        "name": name,
        "body": body,
        "is_default": true
    });
    engine.save_signature(&json.to_string())
}

fn unix_now_cli() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cmd_schedule(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let account = flag(args, "--account").ok_or("missing --account")?;
    let to = flag(args, "--to").ok_or("missing --to")?;
    let at: i64 = flag(args, "--at").ok_or("missing --at")?.parse().map_err(|_| "bad --at")?;
    let req = SendRequest {
        account_id: account.into(),
        to: vec![to],
        subject: flag(args, "--subject").unwrap_or_default(),
        body_text: flag(args, "--body").unwrap_or_default(),
        ..SendRequest::default()
    };
    let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    engine.schedule_send(&json, at)
}

fn cmd_rule_add(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let account = flag(args, "--account").ok_or("missing --account")?;
    let name = flag(args, "--name").unwrap_or_else(|| "rule".into());
    let subject = flag(args, "--subject").unwrap_or_default();
    let json = serde_json::json!({
        "id": "",
        "account_id": account,
        "name": name,
        "enabled": true,
        "definition": {
            "match_all": false,
            "conditions": [{"field": "subject", "contains": subject}],
            "actions": [{"type": "flag", "seen": true}]
        }
    });
    engine.add_rule(&json.to_string())
}

fn account_from_flags(args: &[String]) -> Result<AddAccountRequest, String> {
    let email = flag(args, "--email").ok_or("missing --email")?;
    let mut req = AddAccountRequest::new(email);
    req.password = flag(args, "--password");
    req.access_token = flag(args, "--access-token");
    req.refresh_token = flag(args, "--refresh-token");
    req.display_name = flag(args, "--name");
    req.imap_host = flag(args, "--host");
    req.imap_port = flag(args, "--port").and_then(|s| s.parse().ok());
    req.smtp_host = flag(args, "--smtp-host");
    req.smtp_port = flag(args, "--smtp-port").and_then(|s| s.parse().ok());
    req.username = flag(args, "--user");
    req.accept_invalid_certs = has_flag(args, "--insecure");
    req.imap_starttls = has_flag(args, "--starttls");
    req.smtp_starttls = has_flag(args, "--smtp-starttls");
    Ok(req)
}

fn parse_db(args: &[String]) -> (PathBuf, Vec<String>, bool) {
    let mut db = default_db();
    let mut rest = Vec::new();
    let mut file_secrets = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--db"
            && let Some(p) = args.get(i + 1)
        {
            db = PathBuf::from(p);
            i += 2;
            continue;
        }
        if args[i] == "--secrets"
            && let Some(p) = args.get(i + 1)
        {
            file_secrets = p == "file";
            i += 2;
            continue;
        }
        rest.push(args[i].clone());
        i += 1;
    }
    (db, rest, file_secrets)
}

fn split_addrs(raw: &str) -> Vec<String> {
    raw.split([',', ';'])
        .map(|part| {
            let trimmed = part.trim();
            if let (Some(start), Some(end)) = (trimmed.rfind('<'), trimmed.rfind('>'))
                && start < end
            {
                trimmed[start + 1..end].trim().to_string()
            } else {
                trimmed.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn cmd_ui(engine: &FfiEngine, args: &[String]) -> Result<String, String> {
    let accounts: serde_json::Value =
        serde_json::from_str(&engine.list_accounts()?).map_err(|e| e.to_string())?;
    let Some(acc) = accounts.as_array().and_then(|a| a.first()) else {
        return Ok("no accounts — run: imyemail-cloud add --email …".into());
    };
    let account_id = acc["id"].as_str().unwrap_or_default();
    let folders: serde_json::Value =
        serde_json::from_str(&engine.list_folders(account_id)?).map_err(|e| e.to_string())?;
    let folder_id = flag(args, "--folder")
        .or_else(|| {
            folders.as_array().and_then(|fs| {
                fs.iter()
                    .find(|f| f["role"] == "inbox")
                    .or_else(|| fs.first())
                    .and_then(|f| f["id"].as_str().map(ToString::to_string))
            })
        })
        .ok_or("no folders")?;
    let _ = engine.sync_folder(&folder_id);
    let msgs: serde_json::Value = serde_json::from_str(&engine.list_messages(&folder_id, 0, 20)?)
        .map_err(|e| e.to_string())?;
    let selected = flag(args, "--message").or_else(|| {
        msgs.as_array()
            .and_then(|m| m.first())
            .and_then(|m| m["id"].as_str().map(ToString::to_string))
    });
    let body =
        if let Some(id) = &selected { engine.body(id).unwrap_or_default() } else { String::new() };
    Ok(render_three_pane(acc, &folders, &msgs, selected.as_deref(), &body))
}

fn render_three_pane(
    acc: &serde_json::Value,
    folders: &serde_json::Value,
    msgs: &serde_json::Value,
    selected: Option<&str>,
    body_json: &str,
) -> String {
    let mut nav = String::from("A 导航\n");
    nav.push_str(&format!("  {}\n", acc["email"].as_str().unwrap_or("")));
    if let Some(fs) = folders.as_array() {
        for f in fs {
            nav.push_str(&format!(
                "  · {} [{}]\n",
                f["path"].as_str().unwrap_or("?"),
                f["role"].as_str().unwrap_or("")
            ));
        }
    }
    let mut list = String::from("B 列表\n");
    if let Some(ms) = msgs.as_array() {
        for m in ms {
            let mark = if Some(m["id"].as_str().unwrap_or("")) == selected { ">" } else { " " };
            let from = m["from"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["email"].as_str())
                .unwrap_or("");
            list.push_str(&format!("{mark} {}  {}\n", from, m["subject"].as_str().unwrap_or("")));
        }
    }
    let mut read = String::from("C 阅读\n");
    if let Ok(body) = serde_json::from_str::<serde_json::Value>(body_json) {
        read.push_str(
            body["text"].as_str().unwrap_or(body["html_sanitized"].as_str().unwrap_or("")),
        );
    } else if !body_json.is_empty() {
        read.push_str(body_json);
    }
    format!(
        "imyemail-cloud  三栏（docs/09）\n\
         ────────────────────────────────────────\n\
         {nav}\n\
         {list}\n\
         {read}\n"
    )
}

fn usage() -> &'static str {
    "imyemail-cloud — imyemail-cloud CLI\n\
     \n\
     --db PATH          database (or IMYEMAIL_CLOUD_DB)\n\
     --secrets file|os  Apple 默认 os（Keychain）；file 为 sidecar\n\
      probe --email E [--host H --port P --password PW --access-token T --insecure]\n\
       add    --email E [--host H --port P --password PW --access-token T --refresh-token R --insecure]\n\
       oauth-start --email E\n\
       oauth-complete --email E --code C --verifier V [--redirect URI]\n\
       providers\n\
      accounts\n\
     folders --account ID\n\
     sync --folder ID\n\
     list --folder ID\n\
     read --message ID\n\
      draft --account ID --to ADDR [--cc C --bcc B --subject S --body B]\n\
       send  --account ID --to ADDR [--cc C --bcc B --subject S --body B --undo SECS]\n\
      flush [--now UNIX]\n\
      undo --draft ID\n\
      clear-failed-queue (failed sends become drafts; no resend)\n\
      signatures --account ID\n\
      sig-add --account ID --name N --body B\n\
      attachments --message ID\n\
      attach --id ID\n\
     flags --message ID --json FLAGS\n\
     delete --message ID\n\
     outbox\n\
      events\n\
      tick [--account ID] [--idle] [--wait-ms MS]\n\
      ui [--folder ID --message ID]\n\
     search --q TEXT\n\
      inbox\n\
      thread --thread ID\n\
      contacts [--account ID]\n\
      vcard [--account ID] --data VCARD\n\
      csv [--account ID] --data CSV\n\
      ics --data ICS\n\
      rsvp --data ICS [--partstat accept|decline|tentative]\n\
      diagnose --email E [--host H --port P --password PW --insecure]\n\
      backup\n\
      restore --data JSON\n\
      ffi --method NAME [--args JSON]\n"
}

pub fn db_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}
