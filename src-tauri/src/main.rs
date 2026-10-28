#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;

use std::sync::Mutex;
use store::{Snapshot, Store, TransactionInput};
use tauri::{Manager, State};

struct AppState(Mutex<Store>);

fn access<T>(state: State<'_, AppState>, f: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
    let mut store = state.0.lock().map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
    f(&mut store)
}

#[tauri::command]
fn snapshot(state: State<'_, AppState>, month: String) -> Result<Snapshot, String> {
    access(state, |s| s.snapshot(&month))
}

#[tauri::command]
fn save_transaction(state: State<'_, AppState>, input: TransactionInput) -> Result<(), String> {
    access(state, |s| s.save(&input))
}

#[tauri::command]
fn delete_transaction(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    access(state, |s| s.delete(id))
}

#[tauri::command]
fn add_account(state: State<'_, AppState>, name: String) -> Result<(), String> {
    access(state, |s| s.add_account(&name))
}

#[tauri::command]
fn export_csv(state: State<'_, AppState>) -> Result<String, String> {
    access(state, |s| s.export_csv())
}

#[tauri::command]
fn import_csv(state: State<'_, AppState>, content: String) -> Result<store::ImportResult, String> {
    access(state, |s| s.import_csv(&content))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let store = Store::open(&dir.join("paisapilot.sqlite3"))
                .map_err(std::io::Error::other)?;
            app.manage(AppState(Mutex::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![snapshot, save_transaction, delete_transaction, add_account, export_csv, import_csv])
        .run(tauri::generate_context!())
        .expect("PaisaPilot could not start");
}
