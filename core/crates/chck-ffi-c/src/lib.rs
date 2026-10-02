//! C ABI：`chck_mail_open` / `chck_mail_call` / `chck_mail_free` / `chck_mail_close`。
//! 各端只走 JSON 字符串，不接触 IMAP/SQL。

#[cfg(target_os = "android")]
mod android;

use chck_ffi::FfiEngine;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

const ERR_OK: c_int = 0;
const ERR_INVALID: c_int = 1;
const ERR_ENGINE: c_int = 2;

/// # Safety
/// `path` must be null or a valid NUL-terminated UTF-8 string. `out_err` must
/// be null or writable pointer storage. The caller owns the returned handle and error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chck_mail_open(
    path: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut FfiEngine {
    unsafe { open_handle(path, out_err, false) }
}

/// Opens an engine whose credentials are stored beside the database. Intended
/// for isolated tests and migrations, not the
/// default Apple client path.
///
/// # Safety
/// `path` must be null or a valid NUL-terminated UTF-8 string. `out_err` must
/// be null or writable pointer storage. The caller owns the returned handle and error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chck_mail_open_with_file_secrets(
    path: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut FfiEngine {
    unsafe { open_handle(path, out_err, true) }
}

unsafe fn open_handle(
    path: *const c_char,
    out_err: *mut *mut c_char,
    file_secrets: bool,
) -> *mut FfiEngine {
    clear_err(out_err);
    let Some(path) = cstr(path) else {
        set_err(out_err, "missing path");
        return ptr::null_mut();
    };
    let opened =
        if file_secrets { FfiEngine::open_with_file_secrets(&path) } else { open_engine(&path) };
    match opened {
        Ok(engine) => Box::into_raw(Box::new(engine)),
        Err(e) => {
            set_err(out_err, &e);
            ptr::null_mut()
        }
    }
}

/// # Safety
/// `engine` must be null or a live handle from `chck_mail_open`, and must not
/// be closed during this call. String arguments must be null or valid NUL-terminated
/// strings; `out_err` must be null or writable pointer storage.
/// Cache-only `cached_body` calls may run concurrently with other calls. Each
/// call must use its own output/error storage, and closing must wait for all calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chck_mail_call(
    engine: *mut FfiEngine,
    method: *const c_char,
    args_json: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    clear_err(out_err);
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        set_err(out_err, "null engine");
        return ptr::null_mut();
    };
    let Some(method) = cstr(method) else {
        set_err(out_err, "missing method");
        return ptr::null_mut();
    };
    let args = cstr(args_json).unwrap_or_default();
    match engine.call(&method, &args) {
        Ok(json) => to_cstring(&json),
        Err(e) => {
            set_err(out_err, &e);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn chck_mail_last_status(err: *const c_char) -> c_int {
    if err.is_null() { ERR_OK } else { ERR_ENGINE }
}

/// # Safety
/// `ptr` must be null or an unfreed string returned by this library. It must
/// not be accessed after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chck_mail_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

/// # Safety
/// `engine` must be null or an unclosed handle from `chck_mail_open`. No
/// calls may be in flight, and the handle must not be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chck_mail_close(engine: *mut FfiEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

fn open_engine(path: &str) -> Result<FfiEngine, String> {
    #[cfg(target_os = "android")]
    {
        if std::env::var("IMYEMAIL_CLOUD_SECRETS").ok().as_deref() == Some("file") {
            return FfiEngine::open_with_file_secrets(path);
        }
        FfiEngine::open_with_os_secrets(path, Box::new(android::AndroidSecretStore))
    }
    #[cfg(not(target_os = "android"))]
    {
        FfiEngine::open(path)
    }
}

fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok().map(str::to_string)
}

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', "")).map(CString::into_raw).unwrap_or(ptr::null_mut())
}

fn clear_err(out_err: *mut *mut c_char) {
    if !out_err.is_null() {
        unsafe {
            *out_err = ptr::null_mut();
        }
    }
}

fn set_err(out_err: *mut *mut c_char, msg: &str) {
    if !out_err.is_null() {
        unsafe {
            *out_err = to_cstring(msg);
        }
    }
    let _ = ERR_INVALID;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn cached_body_miss_returns_json_null_not_a_null_pointer() {
        let path = CString::new(":memory:").unwrap();
        let method = CString::new("cached_body").unwrap();
        let args = CString::new(r#"{"message_id":"missing"}"#).unwrap();
        let mut error = ptr::null_mut();
        let engine = unsafe { chck_mail_open(path.as_ptr(), &raw mut error) };
        assert!(!engine.is_null(), "{}", display_err(error));
        let result =
            unsafe { chck_mail_call(engine, method.as_ptr(), args.as_ptr(), &raw mut error) };
        assert!(error.is_null(), "{}", display_err(error));
        assert!(!result.is_null());
        assert_eq!(unsafe { CStr::from_ptr(result) }.to_str().unwrap(), "null");
        unsafe {
            chck_mail_free(result);
            chck_mail_close(engine);
        }
    }

    #[test]
    fn open_call_close() {
        let dir = std::env::temp_dir().join(format!("chck-ffi-c-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let path = CString::new(db.to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        let engine = unsafe { chck_mail_open(path.as_ptr(), &raw mut err) };
        assert!(!engine.is_null(), "{}", display_err(err));
        let method = CString::new("list_providers").unwrap();
        let args = CString::new("{}").unwrap();
        let json = unsafe { chck_mail_call(engine, method.as_ptr(), args.as_ptr(), &raw mut err) };
        assert!(!json.is_null(), "{}", display_err(err));
        let text = unsafe { CStr::from_ptr(json) }.to_string_lossy().into_owned();
        assert!(text.contains("imap.qq.com"));
        unsafe {
            chck_mail_free(json);
            chck_mail_close(engine);
        }
    }

    #[test]
    fn explicit_file_secrets_open_is_isolated() {
        let dir = std::env::temp_dir().join(format!("chck-ffi-c-file-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("mail.db");
        let path = CString::new(db.to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        let engine = unsafe { chck_mail_open_with_file_secrets(path.as_ptr(), &raw mut err) };
        assert!(!engine.is_null(), "{}", display_err(err));
        let method = CString::new("list_providers").unwrap();
        let args = CString::new("{}").unwrap();
        let json = unsafe { chck_mail_call(engine, method.as_ptr(), args.as_ptr(), &raw mut err) };
        assert!(!json.is_null(), "{}", display_err(err));
        let value = unsafe { CStr::from_ptr(json) }.to_str().unwrap();
        assert!(value.contains("imap.qq.com"), "{value}");
        unsafe {
            chck_mail_free(json);
            chck_mail_close(engine);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn display_err(err: *mut c_char) -> String {
        if err.is_null() {
            return "ok".into();
        }
        unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned()
    }
}
