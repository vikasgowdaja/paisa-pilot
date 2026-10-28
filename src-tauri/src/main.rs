#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gmail;
mod store;

use std::sync::{Arc, Mutex};
use store::{
    GmailApprovalInput, GmailCandidate, GmailSyncResult, Snapshot, Store, TransactionInput,
};
use tauri::{Manager, State};

struct AppState(Arc<Mutex<Store>>);

fn access<T>(
    state: State<'_, AppState>,
    f: impl FnOnce(&mut Store) -> Result<T, String>,
) -> Result<T, String> {
    let mut store = state
        .0
        .lock()
        .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
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

#[tauri::command]
fn gmail_auth_status() -> Result<gmail::AuthStatus, String> {
    gmail::auth_status()
}

#[tauri::command]
async fn gmail_connect(state: State<'_, AppState>) -> Result<String, String> {
    let store = Arc::clone(&state.0);
    let client_id = gmail::configured_client_id()?.to_string();
    let email = tauri::async_runtime::spawn_blocking(move || gmail::connect(&client_id))
        .await
        .map_err(|e| format!("Google sign-in task failed: {e}"))??;
    store
        .lock()
        .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?
        .reset_gmail_sync()?;
    Ok(email)
}

#[tauri::command]
async fn gmail_sync(state: State<'_, AppState>) -> Result<GmailSyncResult, String> {
    let store = Arc::clone(&state.0);
    let last_sync_ms = {
        let guard = store
            .lock()
            .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
        guard.gmail_last_sync_ms()?
    };
    let client_id = gmail::configured_client_id()?.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let query = gmail::search_after_ms(last_sync_ms);
        let mut page_token = None;
        let mut result = GmailSyncResult {
            added: 0,
            skipped: 0,
        };
        loop {
            let (message_ids, next_page) =
                gmail::list_message_ids(&client_id, &query, page_token.as_deref())?;
            for message_id in message_ids {
                match gmail::fetch_message(&client_id, &message_id)? {
                    Some(message) => {
                        let input = store::GmailMessageInput {
                            message_id: message.message_id,
                            subject: message.subject,
                            sender: message.sender,
                            received_at_ms: message.received_at_ms,
                            transaction_date: message.extracted.date,
                            amount: message.extracted.amount,
                            currency: message.extracted.currency,
                            event_type: message.extracted.event_type,
                            kind: message.extracted.kind,
                            merchant: message.extracted.merchant,
                            account_hint: message.extracted.account_hint,
                            reference: message.extracted.reference,
                            description: message.extracted.description,
                        };
                        let mut guard = store.lock().map_err(|_| {
                            "Database lock unavailable. Restart PaisaPilot.".to_string()
                        })?;
                        if guard.record_gmail_message(&input)? {
                            result.added += 1;
                        } else {
                            result.skipped += 1;
                        }
                    }
                    None => result.skipped += 1,
                }
            }
            match next_page {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }
        let mut guard = store
            .lock()
            .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
        guard.set_gmail_last_sync_ms(chrono::Utc::now().timestamp_millis())?;
        Ok(result)
    })
    .await
    .map_err(|e| format!("Gmail sync task failed: {e}"))?
}

#[tauri::command]
fn gmail_candidates(state: State<'_, AppState>) -> Result<Vec<GmailCandidate>, String> {
    access(state, |store| store.gmail_candidates())
}

#[tauri::command]
fn approve_gmail_candidate(
    state: State<'_, AppState>,
    input: GmailApprovalInput,
) -> Result<bool, String> {
    access(state, |store| store.approve_gmail_candidate(&input))
}

#[tauri::command]
fn ignore_gmail_candidate(state: State<'_, AppState>, candidate_id: i64) -> Result<(), String> {
    access(state, |store| store.ignore_gmail_candidate(candidate_id))
}

#[tauri::command]
fn gmail_disconnect() -> Result<(), String> {
    gmail::disconnect()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let store =
                Store::open(&dir.join("paisapilot.sqlite3")).map_err(std::io::Error::other)?;
            app.manage(AppState(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_transaction,
            delete_transaction,
            add_account,
            export_csv,
            import_csv,
            gmail_auth_status,
            gmail_connect,
            gmail_sync,
            gmail_candidates,
            approve_gmail_candidate,
            ignore_gmail_candidate,
            gmail_disconnect
        ])
        .run(tauri::generate_context!())
        .expect("PaisaPilot could not start");
}
