//! Thin platform layer: everything the UI needs from the host OS that is not plain egui.
//!
//! On Android these calls go through JNI to the Kotlin `MainActivity`; on desktop they are
//! no-ops (the desktop app opens a player process and uses egui text fields directly).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Result of the native text-entry dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextResult {
    pub id: String,
    pub text: String,
    pub submitted: bool,
}

static TEXT_QUEUE: Mutex<Vec<TextResult>> = Mutex::new(Vec::new());
static PLAYER_CLOSED: AtomicBool = AtomicBool::new(false);
static WAKE: OnceLock<egui::Context> = OnceLock::new();

/// Remember the egui context so native callbacks can request a repaint.
pub fn set_wake(ctx: egui::Context) {
    let _ = WAKE.set(ctx);
}

fn wake() {
    if let Some(ctx) = WAKE.get() {
        ctx.request_repaint();
    }
}

/// Called (from JNI) when the user finished the native text dialog.
pub fn push_text(id: String, text: String, submitted: bool) {
    if let Ok(mut q) = TEXT_QUEUE.lock() {
        q.push(TextResult { id, text, submitted });
    }
    wake();
}

/// Called (from JNI) when the in-app player screen was closed.
pub fn notify_player_closed() {
    PLAYER_CLOSED.store(true, Ordering::SeqCst);
    wake();
}

pub fn poll_text() -> Vec<TextResult> {
    TEXT_QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
}

pub fn take_player_closed() -> bool {
    PLAYER_CLOSED.swap(false, Ordering::SeqCst)
}

/// True on devices driven by touch (Android). The UI switches to touch-first behaviour.
pub fn is_touch_platform() -> bool {
    cfg!(target_os = "android")
}

#[cfg(test)]
pub static LAST_TEXT_REQUEST: Mutex<Option<(String, String, String, bool)>> = Mutex::new(None);

/// Open the native text-entry dialog. The answer arrives later through `poll_text`.
pub fn request_text(id: &str, hint: &str, initial: &str, password: bool) {
    #[cfg(test)]
    {
        if let Ok(mut l) = LAST_TEXT_REQUEST.lock() {
            *l = Some((id.to_string(), hint.to_string(), initial.to_string(), password));
        }
    }
    #[cfg(feature = "mobile")]
    if let Err(e) = crate::jni_bridge::show_text_dialog(id, hint, initial, password) {
        log::warn!("text dialog failed: {e}");
    }
    #[cfg(not(feature = "mobile"))]
    let _ = (id, hint, initial, password);
}

/// Play a stream in the Android in-app player. Returns an error message on failure.
pub fn play_video(url: &str, headers: &[(String, String)], title: &str) -> Result<(), String> {
    #[cfg(feature = "mobile")]
    {
        crate::jni_bridge::play_video(url, headers, title)
    }
    #[cfg(not(feature = "mobile"))]
    {
        let _ = (url, headers, title);
        Err("In-app playback is only available in the Android app.".to_string())
    }
}

/// Send the app to the background (Android back button at the root screen).
pub fn move_task_to_back() {
    #[cfg(feature = "mobile")]
    if let Err(e) = crate::jni_bridge::move_to_back() {
        log::warn!("move to back failed: {e}");
    }
}

pub fn copy_text(text: &str) {
    #[cfg(feature = "mobile")]
    if let Err(e) = crate::jni_bridge::copy_text(text) {
        log::warn!("copy failed: {e}");
    }
    #[cfg(not(feature = "mobile"))]
    let _ = text;
}
