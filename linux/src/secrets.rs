//! libsecret（Secret Service）。`--features secret` 时作为 `SecretBackend` 主存储，sidecar 仅迁移/回退。

use chck_engine::SecretBackend;
use chck_types::EngineError;

pub const SCHEMA_NAME: &str = "email.imy.cloud";

pub struct LibsecretBackend;

#[must_use]
pub fn available() -> bool {
    cfg!(feature = "secret")
}

#[cfg(feature = "secret")]
pub fn store(account_id: &str, secret: &str) -> Result<(), String> {
    use gtk4::gio;
    let schema = libsecret::Schema::new(
        SCHEMA_NAME,
        libsecret::SchemaFlags::NONE,
        std::collections::HashMap::from([("account", libsecret::SchemaAttributeType::String)]),
    );
    let attrs = std::collections::HashMap::from([("account", account_id)]);
    libsecret::password_store_sync(
        Some(&schema),
        attrs,
        Some(libsecret::COLLECTION_DEFAULT),
        "imyemail-cloud",
        secret,
        None::<&gio::Cancellable>,
    )
    .map_err(|e| e.to_string())
}

#[cfg(feature = "secret")]
pub fn lookup(account_id: &str) -> Result<Option<String>, String> {
    use gtk4::gio;
    let schema = libsecret::Schema::new(
        SCHEMA_NAME,
        libsecret::SchemaFlags::NONE,
        std::collections::HashMap::from([("account", libsecret::SchemaAttributeType::String)]),
    );
    let attrs = std::collections::HashMap::from([("account", account_id)]);
    match libsecret::password_lookup_sync(Some(&schema), attrs, None::<&gio::Cancellable>) {
        Ok(pw) => Ok(pw.map(|s| s.to_string())),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(feature = "secret")]
pub fn remove(account_id: &str) -> Result<(), String> {
    use gtk4::gio;
    let schema = libsecret::Schema::new(
        SCHEMA_NAME,
        libsecret::SchemaFlags::NONE,
        std::collections::HashMap::from([("account", libsecret::SchemaAttributeType::String)]),
    );
    let attrs = std::collections::HashMap::from([("account", account_id)]);
    libsecret::password_clear_sync(Some(&schema), attrs, None::<&gio::Cancellable>)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "secret"))]
pub fn store(_account_id: &str, _secret: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(feature = "secret"))]
#[allow(dead_code)]
pub fn lookup(_account_id: &str) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(feature = "secret"))]
#[allow(dead_code)]
pub fn remove(_account_id: &str) -> Result<(), String> {
    Ok(())
}

impl SecretBackend for LibsecretBackend {
    fn get(&self, id: &str) -> Option<String> {
        lookup(id).ok().flatten()
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        store(id, secret).map_err(EngineError::Storage)
    }

    fn delete(&self, id: &str) -> Result<(), EngineError> {
        remove(id).map_err(EngineError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_feature() {
        assert_eq!(available(), cfg!(feature = "secret"));
        assert_eq!(SCHEMA_NAME, "email.imy.cloud");
    }
}
