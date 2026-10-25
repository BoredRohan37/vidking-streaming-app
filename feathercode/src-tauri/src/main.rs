#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aichat;
mod db;
mod fsops;
mod gitops;
mod sshops;
mod terminal;

use db::Db;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = Connection::open(dir.join("feathercode.db"))?;
            db::init_schema(&conn)?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(terminal::PtyState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings db
            db::db_list_providers,
            db::db_save_provider,
            db::db_delete_provider,
            db::db_list_workspaces,
            db::db_save_workspace,
            db::db_delete_workspace,
            db::db_get_setting,
            db::db_set_setting,
            // local filesystem
            fsops::fs_list,
            fsops::fs_read,
            fsops::fs_write,
            fsops::fs_create,
            fsops::fs_delete,
            fsops::fs_rename,
            fsops::fs_search,
            // remote ssh workspaces
            sshops::ssh_test,
            sshops::ssh_list,
            sshops::ssh_read,
            sshops::ssh_write,
            sshops::ssh_create,
            sshops::ssh_delete,
            sshops::ssh_rename,
            // git
            gitops::git_clone,
            // ai
            aichat::ai_chat,
            // terminal
            terminal::pty_create,
            terminal::pty_write,
            terminal::pty_resize,
            terminal::pty_kill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
