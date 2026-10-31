//! Mobile shell: runs the shared UI through eframe (GPU / OpenGL ES) instead of the
//! desktop software renderer. Used by the Android app; also compiles on desktop so it can be
//! type-checked without an Android toolchain (`--features mobile-host`).

use crate::app::App;
use crate::data::Backend;
use crate::store::Store;
use crate::{platform, theme};
use std::sync::{mpsc, Arc};

struct Shell {
    app: App,
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

/// Start the app with the given eframe options.
pub fn run(options: eframe::NativeOptions) -> eframe::Result {
    eframe::run_native(
        "MovieBox",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            platform::set_wake(cc.egui_ctx.clone());
            let wake_ctx = cc.egui_ctx.clone();
            let (tx, rx) = mpsc::channel();
            let backend = Backend::new(tx, Arc::new(move || wake_ctx.request_repaint()), true);
            let mut app = App::new(cc.egui_ctx.clone(), backend, rx, Store::load());
            app.touch = true;
            Ok(Box::new(Shell { app }))
        }),
    )
}

/// Entry used by `android_main`.
#[cfg(target_os = "android")]
pub fn run_android(android_app: winit::platform::android::activity::AndroidApp) {
    // Point the engine's config/cache/download folders at the app's private storage.
    if let Some(dir) = android_app.internal_data_path() {
        let base = dir.to_string_lossy().to_string();
        std::env::set_var("HOME", &base);
        std::env::set_var("XDG_CONFIG_HOME", format!("{base}/config"));
        std::env::set_var("XDG_CACHE_HOME", format!("{base}/cache"));
        std::env::set_var("XDG_DATA_HOME", format!("{base}/data"));
        std::env::set_var("MOVIEBOX_CONFIG_DIR", format!("{base}/config/moviebox"));
        std::env::set_var("TMPDIR", format!("{base}/cache"));
    }
    // SAFETY: the pointers come straight from android-activity and stay valid for the process.
    unsafe {
        if let Err(e) = crate::jni_bridge::init(android_app.vm_as_ptr() as *mut _, android_app.activity_as_ptr() as *mut _) {
            eprintln!("JNI init failed: {e}");
        }
    }
    let options = eframe::NativeOptions {
        android_app: Some(android_app),
        ..Default::default()
    };
    if let Err(e) = run(options) {
        eprintln!("MovieBox exited with an error: {e}");
    }
}
