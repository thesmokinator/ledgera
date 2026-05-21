use crate::{
    journal::{
        files::require_journal_path,
        summary::read_journal_summary,
        transactions::{
            create_transaction_for_settings, delete_transaction_for_settings,
            update_transaction_for_settings,
        },
        types::{JournalSummary, TransactionInput},
    },
    logs,
    settings::read_settings,
};
use tauri::AppHandle;

/// Reads the configured journal and returns parsed transaction blocks.
#[tauri::command]
pub(crate) fn list_transactions(app: AppHandle) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    read_journal_summary(&journal_path)
}

/// Appends a new transaction using the existing journal style where possible.
#[tauri::command]
pub(crate) fn create_transaction(
    app: AppHandle,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = create_transaction_for_settings(&settings, &input);
    if let Err(e) = &result {
        logs::log_event_with_details(
            &app,
            "error",
            "transaction_create_failed",
            "Failed to create transaction",
            Some(e.clone()),
        );
    }
    result
}

/// Replaces an existing transaction block by id.
#[tauri::command]
pub(crate) fn update_transaction(
    app: AppHandle,
    id: String,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = update_transaction_for_settings(&settings, &id, &input);
    if let Err(e) = &result {
        logs::log_event_with_details(
            &app,
            "error",
            "transaction_update_failed",
            "Failed to update transaction",
            Some(e.clone()),
        );
    }
    result
}

/// Removes an existing transaction block by id.
#[tauri::command]
pub(crate) fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = delete_transaction_for_settings(&settings, &id);
    if let Err(e) = &result {
        logs::log_event_with_details(
            &app,
            "error",
            "transaction_delete_failed",
            "Failed to delete transaction",
            Some(e.clone()),
        );
    }
    result
}
