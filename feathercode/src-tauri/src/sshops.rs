use crate::db::{Db, Workspace};
use crate::fsops::FileEntry;
use rusqlite::params;
use ssh2::{OpenFlags, OpenType, RenameFlags, Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use tauri::State;

fn load_workspace(db: &State<'_, Db>, id: &str) -> Result<Workspace, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, kind, local_path, ssh_host, ssh_port, ssh_user, ssh_auth, ssh_secret, ssh_root
         FROM workspaces WHERE id = ?1",
        params![id],
        |r| {
            Ok(Workspace {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                local_path: r.get(3)?,
                ssh_host: r.get(4)?,
                ssh_port: r.get(5)?,
                ssh_user: r.get(6)?,
                ssh_auth: r.get(7)?,
                ssh_secret: r.get(8)?,
                ssh_root: r.get(9)?,
            })
        },
    )
    .map_err(|e| format!("workspace not found: {e}"))
}

fn connect_ws(ws: &Workspace) -> Result<Session, String> {
    let host = ws.ssh_host.clone().ok_or("missing SSH host")?;
    let port = ws.ssh_port.unwrap_or(22);
    let user = ws.ssh_user.clone().ok_or("missing SSH user")?;
    let stream = TcpStream::connect((host.as_str(), port as u16))
        .map_err(|e| format!("cannot connect to {host}:{port} — {e}"))?;
    let sess = Session::new().ok_or("failed to init SSH session")?;
    sess.set_tcp_stream(stream);
    sess.handshake().map_err(|e| format!("SSH handshake failed: {e}"))?;
    let auth = ws.ssh_auth.clone().unwrap_or_else(|| "password".into());
    let secret = ws.ssh_secret.clone().unwrap_or_default();
    if secret.is_empty() {
        return Err("no credentials configured for this workspace".into());
    }
    if auth == "key" {
        sess.userauth_pubkey_file(&user, None, Path::new(&secret), None)
            .map_err(|e| format!("key auth failed: {e}"))?;
    } else {
        sess.userauth_password(&user, &secret)
            .map_err(|e| format!("password auth failed: {e}"))?;
    }
    Ok(sess)
}

fn sort_entries(out: &mut Vec<FileEntry>) {
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

fn sftp_rm(sftp: &Sftp, path: &Path) -> Result<(), String> {
    let stat = sftp.stat(path).map_err(|e| e.to_string())?;
    if stat.is_dir() {
        for entry in sftp.readdir(path).map_err(|e| e.to_string())? {
            let name = entry.0.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name == "." || name == ".." {
                continue;
            }
            sftp_rm(sftp, &entry.0)?;
        }
        sftp.rmdir(path).map_err(|e| e.to_string())
    } else {
        sftp.unlink(path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn ssh_test(workspace: Workspace) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&workspace)?;
        let banner = sess.banner();
        Ok(format!("Connected. Remote banner: {banner}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_list(db: State<'_, Db>, workspace_id: String, path: String) -> Result<Vec<FileEntry>, String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        let entries = sftp.readdir(Path::new(&path)).map_err(|e| format!("cannot list {path}: {e}"))?;
        let mut out: Vec<FileEntry> = entries
            .into_iter()
            .filter(|(p, _)| {
                let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                n != "." && n != ".."
            })
            .map(|(p, st)| FileEntry {
                name: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                path: p.to_string_lossy().to_string(),
                is_dir: st.is_dir(),
                size: st.size.unwrap_or(0),
            })
            .collect();
        sort_entries(&mut out);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_read(db: State<'_, Db>, workspace_id: String, path: String) -> Result<String, String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        let mut file = sftp.open(Path::new(&path)).map_err(|e| format!("cannot open {path}: {e}"))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| format!("cannot read {path}: {e}"))?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_write(db: State<'_, Db>, workspace_id: String, path: String, content: String) -> Result<(), String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        let mut file = sftp
            .open_mode(
                Path::new(&path),
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                0o644,
                OpenType::File,
            )
            .map_err(|e| format!("cannot write {path}: {e}"))?;
        file.write_all(content.as_bytes()).map_err(|e| format!("cannot write {path}: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_create(db: State<'_, Db>, workspace_id: String, path: String, is_dir: bool) -> Result<(), String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        if is_dir {
            sftp.mkdir(Path::new(&path), 0o755).map_err(|e| format!("cannot create {path}: {e}"))?;
        } else {
            let _file = sftp
                .open_mode(Path::new(&path), OpenFlags::WRITE | OpenFlags::CREATE, 0o644, OpenType::File)
                .map_err(|e| format!("cannot create {path}: {e}"))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_delete(db: State<'_, Db>, workspace_id: String, path: String) -> Result<(), String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        sftp_rm(&sftp, Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ssh_rename(db: State<'_, Db>, workspace_id: String, from: String, to: String) -> Result<(), String> {
    let ws = load_workspace(&db, &workspace_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ws(&ws)?;
        let sftp = sess.sftp().map_err(|e| format!("sftp failed: {e}"))?;
        sftp
            .rename(Path::new(&from), Path::new(&to), Some(RenameFlags::OVERWRITE | RenameFlags::NATIVE))
            .map_err(|e| format!("cannot rename: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
