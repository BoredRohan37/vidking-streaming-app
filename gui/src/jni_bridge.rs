//! JNI bridge between the Rust UI and the Kotlin `MainActivity` (package `com.moviebox.app`).
//!
//! Rust -> Kotlin: `showTextDialog`, `playVideo`, `moveToBack`, `copyToClipboard`.
//! Kotlin -> Rust: `nativeOnText`, `nativeOnPlayerClosed` (exported below).

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jobject};
use jni::{JNIEnv, JavaVM};
use std::ffi::c_void;
use std::sync::OnceLock;

static VM: OnceLock<JavaVM> = OnceLock::new();
static ACTIVITY: OnceLock<GlobalRef> = OnceLock::new();

/// # Safety
/// `vm_ptr` must be the process `JavaVM*` and `activity_ptr` a valid `jobject` for the
/// running activity (both come from android-activity's `AndroidApp`).
pub unsafe fn init(vm_ptr: *mut c_void, activity_ptr: *mut c_void) -> Result<(), String> {
    let vm = JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM).map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let activity = JObject::from_raw(activity_ptr as jobject);
    let global = env.new_global_ref(&activity).map_err(|e| e.to_string())?;
    drop(env);
    let _ = VM.set(vm);
    let _ = ACTIVITY.set(global);
    Ok(())
}

fn with_env<R>(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<R>) -> Result<R, String> {
    let vm = VM.get().ok_or("the Java VM is not initialised")?;
    let activity = ACTIVITY.get().ok_or("the activity is not initialised")?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let result = f(&mut env, activity.as_obj());
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
        return Err("a Java exception occurred".to_string());
    }
    result.map_err(|e| e.to_string())
}

pub fn show_text_dialog(id: &str, hint: &str, initial: &str, password: bool) -> Result<(), String> {
    with_env(|env, act| {
        let id = env.new_string(id)?;
        let hint = env.new_string(hint)?;
        let initial = env.new_string(initial)?;
        env.call_method(
            act,
            "showTextDialog",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z)V",
            &[
                JValue::Object(&id),
                JValue::Object(&hint),
                JValue::Object(&initial),
                JValue::Bool(password as jboolean),
            ],
        )?;
        Ok(())
    })
}

pub fn play_video(url: &str, headers: &[(String, String)], title: &str) -> Result<(), String> {
    with_env(|env, act| {
        let url = env.new_string(url)?;
        let title = env.new_string(title)?;
        let string_class = env.find_class("java/lang/String")?;
        let flat = env.new_object_array((headers.len() * 2) as i32, string_class, JObject::null())?;
        for (i, (k, v)) in headers.iter().enumerate() {
            let k = env.new_string(k)?;
            let v = env.new_string(v)?;
            env.set_object_array_element(&flat, (i * 2) as i32, k)?;
            env.set_object_array_element(&flat, (i * 2 + 1) as i32, v)?;
        }
        env.call_method(
            act,
            "playVideo",
            "(Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&url), JValue::Object(&flat), JValue::Object(&title)],
        )?;
        Ok(())
    })
}

pub fn move_to_back() -> Result<(), String> {
    with_env(|env, act| {
        env.call_method(act, "moveToBack", "()V", &[])?;
        Ok(())
    })
}

pub fn copy_text(text: &str) -> Result<(), String> {
    with_env(|env, act| {
        let t = env.new_string(text)?;
        env.call_method(act, "copyToClipboard", "(Ljava/lang/String;)V", &[JValue::Object(&t)])?;
        Ok(())
    })
}

fn read(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s).map(|j| j.into()).unwrap_or_default()
}

#[no_mangle]
pub extern "system" fn Java_com_moviebox_app_MainActivity_nativeOnText(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    text: JString,
    submitted: jboolean,
) {
    let id = read(&mut env, &id);
    let text = read(&mut env, &text);
    crate::platform::push_text(id, text, submitted != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_moviebox_app_MainActivity_nativeOnPlayerClosed(_env: JNIEnv, _class: JClass) {
    crate::platform::notify_player_closed();
}
