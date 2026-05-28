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

fn log_command_result<T>(
    app: &AppHandle,
    result: &Result<T, String>,
    ok_code: &str,
    ok_msg: &str,
    err_code: &str,
    err_msg: &str,
) {
    match result {
        Ok(_) => logs::log_event(app, "info", ok_code, ok_msg),
        Err(error) => logs::log_error(app, err_code, err_msg, error),
    }
}

/// Reads the configured journal and returns parsed transaction blocks
/// along with autocomplete suggestions in a single pass.
#[tauri::command]
pub(crate) fn list_transactions(app: AppHandle) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    read_journal_summary(&journal_path, settings.default_commodity.trim())
}

/// Appends a new transaction using the existing journal style where possible.
#[tauri::command]
pub(crate) fn create_transaction(
    app: AppHandle,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = create_transaction_for_settings(&settings, &input);
    log_command_result(
        &app,
        &result,
        "transaction_created",
        "Transaction created successfully.",
        "transaction_create_failed",
        "Failed to create transaction.",
    );
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
    log_command_result(
        &app,
        &result,
        "transaction_updated",
        "Transaction updated successfully.",
        "transaction_update_failed",
        "Failed to update transaction.",
    );
    result
}

/// Removes an existing transaction block by id.
#[tauri::command]
pub(crate) fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = delete_transaction_for_settings(&settings, &id);
    log_command_result(
        &app,
        &result,
        "transaction_deleted",
        "Transaction deleted successfully.",
        "transaction_delete_failed",
        "Failed to delete transaction.",
    );
    result
}
