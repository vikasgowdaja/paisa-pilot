mod finance;
mod gmail;
mod store;

use std::sync::{Arc, Mutex};
use store::{
    GmailApprovalInput, GmailCandidate, GmailSyncResult, Snapshot, Store, TransactionInput,
};
use tauri::{AppHandle, Manager, State};

struct AppState(Arc<Mutex<Store>>);

#[cfg(target_os = "android")]
struct AndroidGmailAuth(tauri::plugin::PluginHandle<tauri::Wry>);

#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeAuthorization {
    access_token: String,
}

#[cfg(target_os = "android")]
fn android_gmail_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("gmail_auth")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.paisapilot.gmail", "GmailAuthPlugin")?;
            app.manage(AndroidGmailAuth(handle));
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
async fn android_access_token(app: &AppHandle) -> Result<String, String> {
    let handle = app.state::<AndroidGmailAuth>().0.clone();
    let result = handle
        .run_mobile_plugin_async::<NativeAuthorization>("authorize", ())
        .await
        .map_err(|error| format!("Google authorization failed: {error}"))?;
    if result.access_token.is_empty() {
        return Err("Google did not return a Gmail access token.".into());
    }
    Ok(result.access_token)
}

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
fn finance_dashboard(state: State<'_, AppState>, month: String) -> Result<finance::Dashboard, String> {
    let today = chrono::Local::now().format("%Y-%m").to_string();
    access(state, |s| s.dashboard(&month, &today))
}

#[tauri::command]
fn set_salary(state: State<'_, AppState>, month: String, gross: String, deductions: String) -> Result<(), String> {
    access(state, |s| s.set_salary(&month, &gross, &deductions))
}

#[tauri::command]
fn clear_salary(state: State<'_, AppState>, month: String) -> Result<(), String> {
    access(state, |s| s.clear_salary(&month))
}

#[tauri::command]
fn set_budget(state: State<'_, AppState>, category_id: i64, amount: String) -> Result<(), String> {
    access(state, |s| s.set_budget(category_id, &amount))
}

#[tauri::command]
fn save_finance_settings(
    state: State<'_, AppState>,
    budget_warn_pct: i64,
    savings_goal_pct: i64,
    invest_goal_pct: i64,
) -> Result<(), String> {
    access(state, |s| s.save_settings(budget_warn_pct, savings_goal_pct, invest_goal_pct))
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
fn gmail_auth_status(_state: State<'_, AppState>) -> Result<gmail::AuthStatus, String> {
    #[cfg(windows)]
    {
        gmail::auth_status()
    }
    #[cfg(target_os = "android")]
    {
        let email = _state
            .0
            .lock()
            .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?
            .gmail_account_email()?;
        Ok(gmail::AuthStatus {
            connected: !email.is_empty(),
            email,
        })
    }
    #[cfg(not(any(windows, target_os = "android")))]
    {
        Err("Gmail import is supported on Windows and Android.".into())
    }
}

#[tauri::command]
async fn gmail_connect(_app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let store = Arc::clone(&state.0);
    #[cfg(windows)]
    let email = {
        let client_id = gmail::configured_client_id()?.to_string();
        tauri::async_runtime::spawn_blocking(move || gmail::connect(&client_id))
            .await
            .map_err(|e| format!("Google sign-in task failed: {e}"))??
    };
    #[cfg(target_os = "android")]
    let email = {
        let access_token = android_access_token(&_app).await?;
        tauri::async_runtime::spawn_blocking(move || gmail::profile_email(&access_token))
            .await
            .map_err(|e| format!("Google account verification failed: {e}"))??
    };
    #[cfg(not(any(windows, target_os = "android")))]
    let email = return Err("Gmail import is supported on Windows and Android.".into());

    let mut store = store
        .lock()
        .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
    store.set_gmail_account_email(&email)?;
    store.reset_gmail_sync()?;
    Ok(email)
}

#[tauri::command]
async fn gmail_sync(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<GmailSyncResult, String> {
    let store = Arc::clone(&state.0);
    let last_sync_ms = {
        let guard = store
            .lock()
            .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?;
        guard.gmail_last_sync_ms()?
    };
    #[cfg(windows)]
    let (client_id, access_token) = (Some(gmail::configured_client_id()?.to_string()), None);
    #[cfg(target_os = "android")]
    let (client_id, access_token) = (None, Some(android_access_token(&_app).await?));
    #[cfg(not(any(windows, target_os = "android")))]
    return Err("Gmail import is supported on Windows and Android.".into());

    tauri::async_runtime::spawn_blocking(move || {
        sync_gmail_messages(store, last_sync_ms, client_id, access_token)
    })
    .await
    .map_err(|error| format!("Gmail sync task failed: {error}"))?
}

fn sync_gmail_messages(
    store: Arc<Mutex<Store>>,
    last_sync_ms: Option<i64>,
    _client_id: Option<String>,
    access_token: Option<String>,
) -> Result<GmailSyncResult, String> {
    let query = gmail::search_after_ms(last_sync_ms);
    let mut page_token = None;
    let mut result = GmailSyncResult {
        added: 0,
        skipped: 0,
    };
    loop {
        let (message_ids, next_page) = match access_token.as_deref() {
            Some(token) => {
                gmail::list_message_ids_with_token(token, &query, page_token.as_deref())?
            }
            None => {
                #[cfg(windows)]
                {
                    let id = _client_id
                        .as_deref()
                        .ok_or("Google sign-in is not configured in this build.")?;
                    gmail::list_message_ids(id, &query, page_token.as_deref())?
                }
                #[cfg(not(windows))]
                {
                    return Err("Gmail authorization is unavailable.".into());
                }
            }
        };
        for message_id in message_ids {
            let message = match access_token.as_deref() {
                Some(token) => gmail::fetch_message_with_token(token, &message_id)?,
                None => {
                    #[cfg(windows)]
                    {
                        let id = _client_id
                            .as_deref()
                            .ok_or("Google sign-in is not configured in this build.")?;
                        gmail::fetch_message(id, &message_id)?
                    }
                    #[cfg(not(windows))]
                    {
                        return Err("Gmail authorization is unavailable.".into());
                    }
                }
            };
            match message {
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
async fn gmail_disconnect(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    #[cfg(windows)]
    gmail::disconnect()?;
    #[cfg(target_os = "android")]
    {
        let handle = _app.state::<AndroidGmailAuth>().0.clone();
        handle
            .run_mobile_plugin_async::<()>("disconnect", ())
            .await
            .map_err(|error| format!("Could not revoke Gmail access: {error}"))?;
    }
    #[cfg(not(any(windows, target_os = "android")))]
    return Err("Gmail import is supported on Windows and Android.".into());

    state
        .0
        .lock()
        .map_err(|_| "Database lock unavailable. Restart PaisaPilot.".to_string())?
        .set_gmail_account_email("")?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(android_gmail_plugin());

    builder
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
            finance_dashboard,
            set_salary,
            clear_salary,
            set_budget,
            save_finance_settings,
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
