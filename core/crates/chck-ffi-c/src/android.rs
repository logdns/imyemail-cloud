use crate::{chck_mail_call, chck_mail_close, chck_mail_free, chck_mail_open};
use chck_engine::SecretBackend;
use chck_types::EngineError;
use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::sys::{JNI_VERSION_1_6, jint, jlong, jstring};
use jni::{JNIEnv, JavaVM};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::OnceLock;

static JVM: OnceLock<JavaVM> = OnceLock::new();
static CLASS: OnceLock<GlobalRef> = OnceLock::new();

const STORE: &str = "email/imy/cloud/chckcore/AndroidSecretStore";

pub struct AndroidSecretStore;

impl SecretBackend for AndroidSecretStore {
    fn get(&self, id: &str) -> Option<String> {
        call_store("get", "(Ljava/lang/String;)Ljava/lang/String;", &[id]).ok().flatten()
    }

    fn set(&self, id: &str, secret: &str) -> Result<(), EngineError> {
        match call_store(
            "set",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            &[id, secret],
        ) {
            Ok(None) => Ok(()),
            Ok(Some(msg)) => Err(EngineError::Storage(msg)),
            Err(e) => Err(EngineError::Storage(e)),
        }
    }

    fn delete(&self, id: &str) -> Result<(), EngineError> {
        match call_store("delete", "(Ljava/lang/String;)Ljava/lang/String;", &[id]) {
            Ok(None) => Ok(()),
            Ok(Some(msg)) => Err(EngineError::Storage(msg)),
            Err(e) => Err(EngineError::Storage(e)),
        }
    }
}

fn call_store(method: &str, sig: &str, args: &[&str]) -> Result<Option<String>, String> {
    let vm = JVM.get().ok_or_else(|| "jvm".to_string())?;
    let class = CLASS.get().ok_or_else(|| "class".to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let owned: Vec<JString> = args
        .iter()
        .map(|s| env.new_string(*s).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let values: Vec<JValue> = owned.iter().map(JValue::from).collect();
    let result = env.call_static_method(class, method, sig, &values).map_err(|e| e.to_string())?;
    match result.l() {
        Ok(obj) if obj.is_null() => Ok(None),
        Ok(obj) => {
            let jstr = JString::from(obj);
            let s = env.get_string(&jstr).map_err(|e| e.to_string())?;
            Ok(Some(s.into()))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn cache_class(vm: &JavaVM) -> Result<(), String> {
    let mut env = vm.get_env().map_err(|e| e.to_string())?;
    let class = env.find_class(STORE).map_err(|e| e.to_string())?;
    let class = env.new_global_ref(class).map_err(|e| e.to_string())?;
    CLASS.set(class).map_err(|_| "cache".to_string())?;
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jint {
    if let Ok(vm) = unsafe { JavaVM::from_raw(vm) } {
        let _ = cache_class(&vm);
        let _ = JVM.set(vm);
    }
    JNI_VERSION_1_6
}

fn jstring_to_string(env: &mut JNIEnv, value: JString) -> Result<String, String> {
    env.get_string(&value).map(|s| s.into()).map_err(|e| e.to_string())
}

fn to_jstring(env: &mut JNIEnv, value: &str) -> jstring {
    env.new_string(value).map(|s| s.into_raw()).unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_email_imy_cloud_chckcore_NativeBridge_nativeOpen(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jlong {
    let Ok(path) = jstring_to_string(&mut env, path) else {
        return 0;
    };
    let c_path = std::ffi::CString::new(path).ok();
    let Some(c_path) = c_path else {
        return 0;
    };
    let mut err: *mut c_char = ptr::null_mut();
    let engine = unsafe { chck_mail_open(c_path.as_ptr(), &raw mut err) };
    if !err.is_null() {
        unsafe {
            chck_mail_free(err);
        }
    }
    engine as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_email_imy_cloud_chckcore_NativeBridge_nativeCall(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    method: JString,
    args: JString,
) -> jstring {
    if handle == 0 {
        return to_jstring(&mut env, r#"{"error":"null engine"}"#);
    }
    let Ok(method) = jstring_to_string(&mut env, method) else {
        return to_jstring(&mut env, r#"{"error":"method"}"#);
    };
    let args = jstring_to_string(&mut env, args).unwrap_or_else(|_| "{}".into());
    let c_method = std::ffi::CString::new(method).unwrap_or_default();
    let c_args = std::ffi::CString::new(args).unwrap_or_default();
    let mut err: *mut c_char = ptr::null_mut();
    let json = unsafe {
        chck_mail_call(handle as *mut _, c_method.as_ptr(), c_args.as_ptr(), &raw mut err)
    };
    if json.is_null() {
        let message = if err.is_null() {
            "call failed".into()
        } else {
            let text = unsafe { std::ffi::CStr::from_ptr(err) }.to_string_lossy().into_owned();
            unsafe {
                chck_mail_free(err);
            }
            text
        };
        return to_jstring(&mut env, &format!(r#"{{"error":{}}}"#, serde_json_string(&message)));
    }
    let text = unsafe { std::ffi::CStr::from_ptr(json) }.to_string_lossy().into_owned();
    unsafe {
        chck_mail_free(json);
    }
    to_jstring(&mut env, &text)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_email_imy_cloud_chckcore_NativeBridge_nativeClose(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            chck_mail_close(handle as *mut _);
        }
    }
}

fn serde_json_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}
