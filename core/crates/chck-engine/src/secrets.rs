use chck_types::EngineError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub trait SecretBackend: Send + Sync {
    fn get(&self, id: &str) -> Option<String>;
    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError>;
    fn delete(&self, id: &str) -> Result<(), EngineError>;
    fn snapshot(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

pub struct FileSecretStore {
    path: Option<PathBuf>,
    mem: Mutex<HashMap<String, String>>,
}

impl FileSecretStore {
    #[must_use]
    pub fn memory() -> Self {
        Self { path: None, mem: Mutex::new(HashMap::new()) }
    }

    #[must_use]
    pub fn open_beside(db: &Path) -> Self {
        if db == Path::new(":memory:") {
            return Self::memory();
        }
        let mut path = db.as_os_str().to_os_string();
        path.push(".secrets");
        let path = PathBuf::from(path);
        let mem = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path: Some(path), mem: Mutex::new(mem) }
    }
}

impl SecretBackend for FileSecretStore {
    fn get(&self, id: &str) -> Option<String> {
        self.mem.lock().ok()?.get(id).cloned()
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        let mut map = self.mem.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        let mut updated = map.clone();
        updated.insert(id.to_string(), secret.to_string());
        persist_secrets(self.path.as_deref(), &updated)?;
        *map = updated;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), EngineError> {
        let mut map = self.mem.lock().map_err(|_| EngineError::Storage("lock".into()))?;
        if !map.contains_key(id) {
            return Ok(());
        }
        let mut updated = map.clone();
        updated.remove(id);
        persist_secrets(self.path.as_deref(), &updated)?;
        *map = updated;
        Ok(())
    }

    fn snapshot(&self) -> HashMap<String, String> {
        self.mem.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub struct KeychainSecretStore {
    service: String,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl KeychainSecretStore {
    #[must_use]
    pub fn new() -> Self {
        Self::with_service("email.imy.cloud")
    }

    #[must_use]
    pub fn with_service(service: impl Into<String>) -> Self {
        Self { service: service.into() }
    }

    fn set_protected(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        use security_framework::access_control::{ProtectionMode, SecAccessControl};
        use security_framework::passwords::{PasswordOptions, set_generic_password_options};
        let mut options = PasswordOptions::new_generic_password(&self.service, id);
        options.set_label("imyemail-cloud");
        let ac = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleAfterFirstUnlock),
            0,
        )
        .map_err(|e| EngineError::Storage(e.to_string()))?;
        options.set_access_control(ac);
        set_generic_password_options(secret.as_bytes(), options)
            .map_err(|e| EngineError::Storage(e.to_string()))
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl Default for KeychainSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl SecretBackend for KeychainSecretStore {
    fn get(&self, id: &str) -> Option<String> {
        security_framework::passwords::get_generic_password(&self.service, id)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        if self.set_protected(id, secret).is_ok() {
            return Ok(());
        }
        security_framework::passwords::set_generic_password(&self.service, id, secret.as_bytes())
            .map_err(|e| EngineError::Storage(e.to_string()))
    }

    fn delete(&self, id: &str) -> Result<(), EngineError> {
        match security_framework::passwords::delete_generic_password(&self.service, id) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == -25300 => Ok(()), // errSecItemNotFound
            Err(error) => Err(EngineError::Storage(error.to_string())),
        }
    }
}

pub struct OverlaySecretStore {
    primary: Box<dyn SecretBackend>,
    fallback: FileSecretStore,
}

impl OverlaySecretStore {
    #[must_use]
    pub fn new(primary: Box<dyn SecretBackend>, fallback: FileSecretStore) -> Self {
        let overlay = Self { primary, fallback };
        overlay.migrate();
        overlay
    }

    fn migrate(&self) {
        for (id, secret) in self.fallback.snapshot() {
            if self.primary.get(&id).is_some() {
                let _ = self.fallback.delete(&id);
                continue;
            }
            if self.primary.set(&id, &secret).is_ok() {
                let _ = self.fallback.delete(&id);
            }
        }
    }
}

impl SecretBackend for OverlaySecretStore {
    fn get(&self, id: &str) -> Option<String> {
        self.primary.get(id).or_else(|| self.fallback.get(id))
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        // The sidecar is a legacy migration source, never a fallback for new
        // credentials when the operating-system vault is unavailable.
        self.primary.set(id, secret)?;
        self.fallback.delete(id)
    }

    fn delete(&self, id: &str) -> Result<(), EngineError> {
        let primary = self.primary.delete(id);
        let fallback = self.fallback.delete(id);
        primary.and(fallback)
    }

    fn snapshot(&self) -> HashMap<String, String> {
        let mut map = self.fallback.snapshot();
        map.extend(self.primary.snapshot());
        map
    }
}

fn persist_secrets(path: Option<&Path>, map: &HashMap<String, String>) -> Result<(), EngineError> {
    let Some(path) = path else { return Ok(()) };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string(map).map_err(|e| EngineError::Storage(e.to_string()))?;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let tmp = PathBuf::from(tmp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let result = (|| {
        let mut file = options.open(&tmp)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|e: std::io::Error| EngineError::Storage(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MapBackend(Mutex<HashMap<String, String>>);

    struct UnavailableBackend;

    impl SecretBackend for UnavailableBackend {
        fn get(&self, _: &str) -> Option<String> {
            None
        }
        fn set(&self, _: &str, _: &str) -> Result<(), EngineError> {
            Err(EngineError::Storage("vault unavailable".into()))
        }
        fn delete(&self, _: &str) -> Result<(), EngineError> {
            Err(EngineError::Storage("vault unavailable".into()))
        }
    }

    #[test]
    fn unavailable_vault_does_not_persist_credentials_or_hide_delete_failure() {
        let dir = std::env::temp_dir().join(format!("chck-vault-{}", uuid::Uuid::new_v4()));
        let db = dir.join("mail.db");
        let overlay = OverlaySecretStore::new(
            Box::new(UnavailableBackend),
            FileSecretStore::open_beside(&db),
        );
        assert!(overlay.set("a1", "test-password").is_err());
        assert!(overlay.get("a1").is_none());
        assert!(!dir.exists());
        assert!(overlay.delete("a1").is_err());
    }

    #[test]
    fn memory_database_never_uses_a_sidecar() {
        let store = FileSecretStore::open_beside(Path::new(":memory:"));
        assert!(store.path.is_none());
        store.set("test-account", "test-password").unwrap();
        assert_eq!(store.get("test-account").as_deref(), Some("test-password"));
    }

    #[test]
    fn failed_persistence_preserves_previous_memory_state() {
        let dir = std::env::temp_dir().join(format!("chck-persist-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = FileSecretStore { path: Some(dir.clone()), mem: Mutex::new(HashMap::new()) };
        assert!(store.set("a1", "test-password").is_err());
        assert!(store.get("a1").is_none());
        std::fs::remove_dir(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_secrets_are_private_from_creation() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("chck-mode-{}", uuid::Uuid::new_v4()));
        let store = FileSecretStore::open_beside(&dir.join("mail.db"));
        store.set("a1", "test-password").unwrap();
        let mode = std::fs::metadata(store.path.as_ref().unwrap()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        store.set("a1", "replacement-password").unwrap();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

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

        fn snapshot(&self) -> HashMap<String, String> {
            self.0.lock().map(|m| m.clone()).unwrap_or_default()
        }
    }

    #[test]
    fn overlay_migrates_sidecar_then_reads_primary() {
        let dir = std::env::temp_dir().join(format!("chck-overlay-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let sidecar = FileSecretStore::open_beside(&db);
        sidecar.set("a1", "from-file").unwrap();
        let overlay =
            OverlaySecretStore::new(Box::new(MapBackend(Mutex::new(HashMap::new()))), sidecar);
        assert_eq!(overlay.get("a1").as_deref(), Some("from-file"));
        let leftover = std::fs::read_to_string(format!("{}.secrets", db.display())).unwrap();
        assert!(!leftover.contains("from-file"), "{leftover}");
        overlay.set("a1", "from-os").unwrap();
        assert_eq!(overlay.get("a1").as_deref(), Some("from-os"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[test]
    fn keychain_backend_roundtrip() {
        let id = format!("t-{}", uuid::Uuid::new_v4());
        let store = KeychainSecretStore::with_service("email.imy.cloud.test");
        let _ = store.delete(&id);
        store.set(&id, "kc-pass").expect("keychain");
        assert_eq!(store.get(&id).as_deref(), Some("kc-pass"));
        store.delete(&id).unwrap();
        assert!(store.get(&id).is_none());
    }
}
