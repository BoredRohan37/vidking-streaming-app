use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub struct PtySession {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
}

#[derive(Default)]
pub struct PtyState(pub Mutex<HashMap<String, PtySession>>);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PtyOut {
    id: String,
    data: String,
}

#[tauri::command]
pub fn pty_create(app: AppHandle, state: State<'_, PtyState>, id: String, cwd: String) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("cannot open pty: {e}"))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(&cwd);

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| format!("cannot spawn shell: {e}"))?;
    let writer = pair.master.take_writer().map_err(|e| format!("pty writer: {e}"))?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| format!("pty reader: {e}"))?;

    state
        .0
        .lock()
        .unwrap()
        .insert(id.clone(), PtySession { writer: Box::new(writer), master: Box::new(pair.master) });

    // Background thread: forward shell output to the frontend
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    let app2 = app.clone();
    let id2 = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = app2.emit(
                        "pty-output",
                        PtyOut { id: id2.clone(), data: String::from_utf8_lossy(&buf[..n]).to_string() },
                    );
                }
                Err(_) => break,
            }
        }
        let _ = app2.emit("pty-closed", serde_json::json!({ "id": id2 }));
    });
    Ok(())
}

#[tauri::command]
pub fn pty_write(state: State<'_, PtyState>, id: String, data: String) -> Result<(), String> {
    let mut map = state.0.lock().unwrap();
    if let Some(sess) = map.get_mut(&id) {
        sess.writer
            .write_all(data.as_bytes())
            .and_then(|_| sess.writer.flush())
            .map_err(|e| e.to_string())
    } else {
        Err("terminal session not found".into())
    }
}

#[tauri::command]
pub fn pty_resize(state: State<'_, PtyState>, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let map = state.0.lock().unwrap();
    if let Some(sess) = map.get(&id) {
        sess.master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())
    } else {
        Err("terminal session not found".into())
    }
}

#[tauri::command]
pub fn pty_kill(state: State<'_, PtyState>, id: String) -> Result<(), String> {
    // Dropping the master closes the pty and terminates the shell
    state.0.lock().unwrap().remove(&id);
    Ok(())
}
