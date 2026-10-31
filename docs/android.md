# MovieBox for Android (phones and tablets)

The Android app reuses the same Rust UI and engine as the desktop app (`../gui`), drawn on the GPU
with OpenGL ES. A small Kotlin layer provides what only Android can: the text-entry dialog, the
in-app video player (Media3 / ExoPlayer, with custom HTTP header support), the clipboard and
the back button.

```
gui/ (Rust)  --cargo-ndk-->  libmoviebox_gui.so  -->  android/app (Kotlin + Gradle)  -->  MovieBox.apk
```

## Build with GitHub Actions (recommended)

1. Push the repository to GitHub.
2. Open **Actions → Android build → Run workflow**.
3. When it finishes, download the **MovieBox-android-apk** artifact, unzip it and install the APK
   on your phone (allow "Install unknown apps" for your file manager or browser).

## Build locally

Requirements: JDK 17, Android SDK 34 + NDK 26.3.11579264, Rust (stable, with the Android targets),
`cargo-ndk`, and Gradle 8.7.

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk --locked
export ANDROID_NDK_HOME="$ANDROID_SDK_ROOT/ndk/26.3.11579264"

( cd gui && cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
    -o ../android/app/src/main/jniLibs build --release --lib --features android-game-activity )

( cd android && gradle assembleRelease )     # APK: android/app/build/outputs/apk/release/
```

The release APK is signed with the debug key so it installs directly. Use your own keystore for
the Play Store.

## How it fits together

| Piece | File |
|---|---|
| Native entry point | `gui/src/lib.rs` (`android_main`) and `gui/src/mobile.rs` |
| Rust ⇄ Kotlin (JNI) | `gui/src/jni_bridge.rs` and `MainActivity.kt` |
| Text input dialog | `MainActivity.showTextDialog` |
| In-app player | `PlayerActivity.kt` |
| Touch/phone UI | `gui/src/mobile_ui.rs`, adaptive layout in `gui/src/pages*.rs` |

Layouts adapt by window width: under 700 dp the phone layout (bottom navigation, 3-column grids,
portrait hero) is used; wider windows (tablets, landscape) use the larger layout with the top
navigation bar. All touch targets are 44 dp or larger.

## Notes

* Downloads are saved inside the app's private storage for now.
* Android TV (remote control navigation) is planned as the next step.
