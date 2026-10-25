use std::path::Path;

/// Shallow-clone a git repository into `dest` (uses system `git`).
#[tauri::command]
pub async fn git_clone(url: String, dest: String) -> Result<String, String> {
    if url.trim().is_empty() || dest.trim().is_empty() {
        return Err("repository URL and destination are required".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if Path::new(&dest).exists() {
            return Err(format!("destination already exists: {dest}"));
        }
        let out = std::process::Command::new("git")
            .args(["clone", "--depth", "1", &url, &dest])
            .output()
            .map_err(|e| format!("git not available: {e}"))?;
        if out.status.success() {
            Ok(dest)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
