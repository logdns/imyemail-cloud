use chck_ffi::FfiEngine;
use serde_json::{Value, json};

fn call(engine: &FfiEngine, method: &str, args: Value) -> Value {
    serde_json::from_str(&engine.call(method, &args.to_string()).unwrap()).unwrap()
}

#[test]
fn settings_roundtrip_preserves_legacy_fields_and_allows_tls_reset() {
    let engine = FfiEngine::open_with_file_secrets(":memory:").unwrap();
    let account = call(
        &engine,
        "add_account",
        json!({"json": json!({
        "email": "user@outlook.com", "password": "private-test-password",
        "imap_host": "custom.example.test", "imap_port": 1993,
        "smtp_host": "submission.example.test", "smtp_port": 1587,
        "username": "login-name", "smtp_starttls": true, "imap_starttls": true,
        "accept_invalid_certs": true, "api_base": "https://api.example.test"
    }).to_string()}),
    );
    let id = account["id"].as_str().unwrap();
    call(
        &engine,
        "update_account",
        json!({"json": json!({"id": id, "email": "user@outlook.com", "display_name": "Renamed"}).to_string()}),
    );
    let settings = call(&engine, "account_settings", json!({"account_id": id}));
    assert_eq!(settings["imap_host"], "custom.example.test");
    assert_eq!(settings["smtp_host"], "submission.example.test");
    assert_eq!(settings["smtp_port"], 1587);
    assert_eq!(settings["smtp_starttls"], true);
    assert!(!settings.to_string().contains("private-test-password"));
    assert!(settings.get("password").is_none());
    call(
        &engine,
        "update_account",
        json!({"json": json!({"id": id, "email": "user@outlook.com", "smtp_starttls": false, "imap_starttls": false, "accept_invalid_certs": false, "password": ""}).to_string()}),
    );
    let settings = call(&engine, "account_settings", json!({"account_id": id}));
    assert_eq!(settings["smtp_starttls"], false);
    assert_eq!(settings["imap_starttls"], false);
    assert_eq!(settings["accept_invalid_certs"], false);
    assert_eq!(settings["api_base"], "https://api.example.test");
}
