use chck_ffi::FfiEngine;
use serde_json::{Value, json};
use std::io::{BufRead, Read, Write};

const MAX_REQUEST: usize = 4 * 1024 * 1024;
const MAX_RESPONSE: usize = 8 * 1024 * 1024;
const METHODS: &[&str] = &[
    "list_accounts",
    "list_providers",
    "add_account",
    "test_account",
    "remove_account",
    "list_folders",
    "sync_folder",
    "unified_inbox",
    "list_messages",
    "body",
    "cached_body",
    "save_draft",
    "send",
    "flush_due_sends",
    "outbox",
    "undo_send",
    "set_flags",
    "search",
];

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn open(path: &str) -> Result<FfiEngine, String> {
    FfiEngine::open_with_strict_os_secrets(
        path,
        Box::new(chck_engine::KeychainSecretStore::with_service("email.imy.cloud.mygo")),
    )
}

#[cfg(all(feature = "desktop-vault", any(target_os = "windows", target_os = "linux")))]
struct DesktopVault;

#[cfg(all(feature = "desktop-vault", any(target_os = "windows", target_os = "linux")))]
impl chck_engine::SecretBackend for DesktopVault {
    fn get(&self, id: &str) -> Option<String> {
        keyring::Entry::new("email.imy.cloud.mygo", id).ok()?.get_password().ok()
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), chck_types::EngineError> {
        keyring::Entry::new("email.imy.cloud.mygo", id)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|_| chck_types::EngineError::Storage("OS credential vault unavailable".into()))
    }

    fn delete(&self, id: &str) -> Result<(), chck_types::EngineError> {
        match keyring::Entry::new("email.imy.cloud.mygo", id)
            .and_then(|entry| entry.delete_credential())
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => {
                Err(chck_types::EngineError::Storage("OS credential vault unavailable".into()))
            }
        }
    }
}

#[cfg(all(feature = "desktop-vault", any(target_os = "windows", target_os = "linux")))]
fn open(path: &str) -> Result<FfiEngine, String> {
    FfiEngine::open_with_strict_os_secrets(path, Box::new(DesktopVault))
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    all(feature = "desktop-vault", any(target_os = "windows", target_os = "linux"))
)))]
fn open(_path: &str) -> Result<FfiEngine, String> {
    Err("desktop-vault feature required".into())
}

pub fn run(path: &str) -> Result<(), String> {
    let engine = open(path)?;
    serve(&engine, std::io::stdin().lock(), std::io::stdout().lock())
        .map_err(|_| "desktop IPC unavailable".into())
}

fn serve(
    engine: &FfiEngine,
    mut input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    loop {
        let mut line = Vec::new();
        let count = input.by_ref().take((MAX_REQUEST + 1) as u64).read_until(b'\n', &mut line)?;
        if count == 0 {
            return Ok(());
        }
        if line.len() > MAX_REQUEST || !line.ends_with(b"\n") {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "request too large"));
        }
        let response = handle(engine, &line);
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
}

fn handle(engine: &FfiEngine, line: &[u8]) -> Value {
    let Ok(request) = serde_json::from_slice::<Value>(line) else {
        return json!({"id": 0, "error": "invalid_request"});
    };
    let id = request.get("id").and_then(Value::as_u64).unwrap_or(0);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(args) = request.get("args").filter(|args| args.is_object()) else {
        return json!({"id": id, "error": "invalid_request"});
    };
    if id == 0 || !METHODS.contains(&method) {
        return json!({"id": id, "error": "method_not_allowed"});
    }
    match engine.call(method, &args.to_string()) {
        Ok(result) if result.len() <= MAX_RESPONSE => {
            match serde_json::from_str::<Value>(&result) {
                Ok(value) => json!({"id": id, "result": value}),
                Err(_) => json!({"id": id, "error": "invalid_response"}),
            }
        }
        _ => json!({"id": id, "error": "operation_failed"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryVault;

    impl chck_engine::SecretBackend for MemoryVault {
        fn get(&self, _id: &str) -> Option<String> {
            None
        }
        fn set(&self, _id: &str, _secret: &str) -> Result<(), chck_types::EngineError> {
            Err(chck_types::EngineError::Storage("fixture vault unavailable".into()))
        }
        fn delete(&self, _id: &str) -> Result<(), chck_types::EngineError> {
            Ok(())
        }
    }

    #[test]
    fn unavailable_vault_never_writes_a_plaintext_fallback() {
        let directory =
            std::env::temp_dir().join(format!("mygo-vault-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let database = directory.join("mail.db");
        let engine = FfiEngine::open_with_strict_os_secrets(
            database.to_str().unwrap(),
            Box::new(MemoryVault),
        )
        .unwrap();
        let result = engine.add_account(r#"{"email":"dev@imyemail.test","password":"fixture-secret","imap_host":"imap.imyemail.test","smtp_host":"smtp.imyemail.test"}"#);
        assert!(result.is_err());
        assert!(!directory.join("mail.db.secrets").exists());
        drop(engine);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn persistent_requests_are_correlated() {
        let engine = FfiEngine::open(":memory:").unwrap();
        let mut output = Vec::new();
        serve(&engine, &b"{\"id\":1,\"method\":\"list_accounts\",\"args\":{}}\n{\"id\":2,\"method\":\"unified_inbox\",\"args\":{}}\n"[..], &mut output).unwrap();
        let responses: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(responses[0], json!({"id": 1, "result": []}));
        assert_eq!(responses[1], json!({"id": 2, "result": []}));
    }

    #[test]
    fn dangerous_and_malformed_requests_are_rejected() {
        let engine = FfiEngine::open(":memory:").unwrap();
        assert_eq!(handle(&engine, b"{")["error"], "invalid_request");
        assert_eq!(
            handle(&engine, br#"{"id":1,"method":"restore","args":{}}"#)["error"],
            "method_not_allowed"
        );
        assert_eq!(
            handle(&engine, br#"{"id":0,"method":"list_accounts","args":{}}"#)["error"],
            "method_not_allowed"
        );
        assert_eq!(
            handle(&engine, br#"{"id":1,"method":"body","args":[]}"#)["error"],
            "invalid_request"
        );
    }

    #[test]
    fn errors_never_echo_secret_inputs() {
        let engine = FfiEngine::open(":memory:").unwrap();
        let request = json!({"id":1,"method":"add_account","args":{"json":"fixture-secret"}});
        let response = handle(&engine, request.to_string().as_bytes());
        assert_eq!(response["error"], "operation_failed");
        assert!(!response.to_string().contains("fixture-secret"));
    }

    #[test]
    fn oversized_or_truncated_lines_end_the_session() {
        let engine = FfiEngine::open(":memory:").unwrap();
        for input in [vec![b'x'; MAX_REQUEST + 1], b"{\"id\":1}".to_vec()] {
            assert!(serve(&engine, &input[..], Vec::new()).is_err());
        }
    }
}
