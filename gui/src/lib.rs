//! MovieBox GUI library: the shared UI and engine glue used by the desktop app
//! (`moviebox-gui`, software rendered) and the Android app (`libmoviebox_gui.so`, GPU).

pub mod ai;
pub mod app;
pub mod art;
pub mod data;
pub mod demo;
pub mod mobile_ui;
pub mod pages;
pub mod pages2;
pub mod platform;
pub mod raster;
pub mod store;
pub mod theme;
pub mod widgets;

#[cfg(feature = "mobile")]
pub mod jni_bridge;
#[cfg(feature = "mobile")]
pub mod mobile;

/// Android entry point (called by GameActivity through android-activity).
#[cfg(all(target_os = "android", feature = "mobile"))]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    mobile::run_android(app);
}

#[cfg(test)]
mod tests;
