//! Application backend for Ledgera.
//!
//! The Rust layer owns journal access, conservative transaction edits, settings
//! persistence, and integration with the official hledger CLI.

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    journal_path: String,
    hledger_path: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    power_user: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HledgerStatus {
    available: bool,
    version: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalSummary {
    path: String,
    transactions: Vec<JournalTransaction>,
    commodities: Vec<String>,
    dashboard: DashboardSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSummary {
    monthly_transactions: Vec<JournalTransaction>,
    scheduled_transactions: Vec<JournalTransaction>,
    active_accounts_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutocompleteSuggestions {
    codes: Vec<String>,
    descriptions: Vec<String>,
    accounts: Vec<String>,
    commodities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalTransaction {
    id: String,
    date: String,
    status: String,
    code: String,
    description: String,
    postings: Vec<JournalPosting>,
    display: TransactionDisplay,
    raw: String,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDisplay {
    account: String,
    amount: String,
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalPosting {
    account: String,
    amount: String,
    commodity: String,
    comment: String,
    raw: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionInput {
    date: String,
    status: String,
    code: String,
    description: String,
    postings: Vec<PostingInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostingInput {
    account: String,
    amount: String,
    #[serde(default)]
    commodity: String,
    #[serde(default)]
    comment: String,
}

#[derive(Debug, Clone)]
struct TransactionBlock {
    transaction: JournalTransaction,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            journal_path: String::new(),
            hledger_path: String::new(),
            theme: default_theme(),
            power_user: false,
        }
    }
}

fn default_theme() -> String {
    "system".to_string()
}

/// Returns persisted application settings from the platform config directory.
#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    read_settings(&app)
}

/// Persists application settings in the platform config directory.
#[tauri::command]
fn update_app_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())?;
    Ok(settings)
}

/// Checks whether the configured hledger executable can be invoked.
#[tauri::command]
fn check_hledger(app: AppHandle) -> Result<HledgerStatus, String> {
    let settings = read_settings(&app)?;
    let executable = hledger_executable(&settings);
    let output = Command::new(&executable).arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(HledgerStatus {
                available: true,
                version: version.clone(),
                message: version,
            })
        }
        Ok(output) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }),
        Err(error) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: error.to_string(),
        }),
    }
}

/// Reads distinct values that can be reused while editing transactions.
#[tauri::command]
fn get_autocomplete_suggestions(app: AppHandle) -> Result<AutocompleteSuggestions, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let content = fs::read_to_string(&journal_path).map_err(|error| error.to_string())?;
    let transactions = parse_transactions(&content);

    Ok(AutocompleteSuggestions {
        codes: unique_sorted(
            transactions
                .iter()
                .map(|transaction| transaction.code.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        descriptions: unique_sorted(
            transactions
                .iter()
                .map(|transaction| transaction.description.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        accounts: unique_sorted(
            transactions
                .iter()
                .flat_map(|transaction| transaction.postings.iter())
                .map(|posting| posting.account.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        commodities: collect_commodities(&transactions),
    })
}

/// Reads the configured journal and returns parsed transaction blocks.
#[tauri::command]
fn list_transactions(app: AppHandle) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let content = fs::read_to_string(&journal_path).map_err(|error| error.to_string())?;
    let transactions = parse_transactions(&content);
    let commodities = collect_commodities(&transactions);

    let dashboard = build_dashboard_summary(&transactions);

    Ok(JournalSummary {
        path: journal_path.to_string_lossy().to_string(),
        transactions,
        commodities,
        dashboard,
    })
}

/// Appends a new transaction using the existing journal style where possible.
#[tauri::command]
fn create_transaction(app: AppHandle, input: TransactionInput) -> Result<JournalSummary, String> {
    mutate_journal(app, |content| {
        let mut updated = content.trim_end_matches(['\r', '\n']).to_string();
        if !updated.is_empty() {
            updated.push_str("\n\n");
        }
        updated.push_str(&format_transaction(&input));
        updated.push('\n');
        Ok(updated)
    })
}

/// Replaces an existing transaction block by id.
#[tauri::command]
fn update_transaction(
    app: AppHandle,
    id: String,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    mutate_journal(app, |content| {
        let lines = split_lines(content);
        let block = find_block(content, &id)?;
        let replacement = format_transaction(&input);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            &replacement,
        ))
    })
}

/// Removes an existing transaction block by id.
#[tauri::command]
fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
    mutate_journal(app, |content| {
        let lines = split_lines(content);
        let block = find_block(content, &id)?;
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            "",
        ))
    })
}

/// Starts the Tauri application and registers backend commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            update_app_settings,
            check_hledger,
            get_autocomplete_suggestions,
            list_transactions,
            create_transaction,
            update_transaction,
            delete_transaction
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Builds the settings file path.
fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("settings.json"))
}

/// Reads settings, returning defaults if no settings file exists yet.
fn read_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

/// Resolves the hledger executable from settings.
fn hledger_executable(settings: &AppSettings) -> String {
    if settings.hledger_path.trim().is_empty() {
        "hledger".to_string()
    } else {
        settings.hledger_path.trim().to_string()
    }
}

/// Resolves the configured journal path.
fn require_journal_path(settings: &AppSettings) -> Result<PathBuf, String> {
    if settings.journal_path.trim().is_empty() {
        return Err(
            "Configure a journal path in Settings before loading transactions.".to_string(),
        );
    }

    let path = PathBuf::from(settings.journal_path.trim());
    if !path.exists() {
        return Err(format!("Journal file does not exist: {}", path.display()));
    }
    Ok(path)
}

/// Applies a journal mutation, validates it, and rolls back on hledger failure.
fn mutate_journal<F>(app: AppHandle, mutate: F) -> Result<JournalSummary, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let original = fs::read_to_string(&journal_path).map_err(|error| error.to_string())?;
    let updated = mutate(&original)?;

    fs::write(&journal_path, &updated).map_err(|error| error.to_string())?;
    if let Err(error) = validate_journal(&settings, &journal_path) {
        fs::write(&journal_path, original).map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    list_transactions(app)
}

/// Runs hledger check for the journal when hledger is available.
fn validate_journal(settings: &AppSettings, journal_path: &Path) -> Result<(), String> {
    let executable = hledger_executable(settings);
    let output = Command::new(&executable)
        .arg("-f")
        .arg(journal_path)
        .arg("check")
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Err(format!("Unable to run hledger check: {}", error)),
    }
}

/// Parses transaction blocks without attempting to reinterpret ledger semantics.
fn parse_transactions(content: &str) -> Vec<JournalTransaction> {
    let lines = split_lines(content);
    let mut transactions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_transaction_header(&lines[index]) {
            index += 1;
            continue;
        }

        let start_line = index + 1;
        let mut end_index = index + 1;
        while end_index < lines.len() && !is_transaction_header(&lines[end_index]) {
            end_index += 1;
        }

        let block_lines = &lines[index..end_index];
        let raw = block_lines.join("\n");
        if let Some(transaction) = parse_transaction_block(start_line, end_index, &raw) {
            transactions.push(transaction);
        }
        index = end_index;
    }

    transactions
}

/// Parses one transaction block.
fn parse_transaction_block(
    start_line: usize,
    end_line: usize,
    raw: &str,
) -> Option<JournalTransaction> {
    let mut lines = raw.lines();
    let header = lines.next()?.trim();
    let (date, rest) = split_first_token(header);
    let mut remaining = rest.trim_start();
    let mut status = String::new();
    let mut code = String::new();

    if remaining.starts_with('*') || remaining.starts_with('!') {
        status = remaining[..1].to_string();
        remaining = remaining[1..].trim_start();
    }

    if remaining.starts_with('(') {
        if let Some(end) = remaining.find(')') {
            code = remaining[..=end].to_string();
            remaining = remaining[end + 1..].trim_start();
        }
    }

    let postings = raw
        .lines()
        .skip(1)
        .filter_map(parse_posting)
        .collect::<Vec<_>>();

    let display = summarize_transaction(&postings);

    Some(JournalTransaction {
        id: format!("line-{}", start_line),
        date: date.to_string(),
        status,
        code,
        description: remaining.to_string(),
        postings,
        display,
        raw: raw.to_string(),
        start_line,
        end_line,
    })
}

/// Parses one posting line while preserving the raw text.
fn parse_posting(line: &str) -> Option<JournalPosting> {
    if line.trim().is_empty() || !line.starts_with(char::is_whitespace) {
        return None;
    }

    let trimmed = line.trim();
    if trimmed.starts_with(';') {
        return None;
    }

    let (posting_content, comment) = split_inline_comment(trimmed);
    let (account, amount) = split_posting_account_amount(posting_content);
    let (quantity, commodity) = parse_posting_amount(amount);
    Some(JournalPosting {
        account: account.trim().to_string(),
        amount: quantity,
        commodity,
        comment: comment.to_string(),
        raw: line.to_string(),
    })
}

/// Parses an hledger amount into quantity and commodity fields.
fn parse_posting_amount(amount: &str) -> (String, String) {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    let Some(number_start) = trimmed.find(|character: char| character.is_ascii_digit()) else {
        return (trimmed.to_string(), String::new());
    };

    let sign = if trimmed[..number_start].contains('-') {
        "-"
    } else {
        ""
    };
    let number_end = trimmed[number_start..]
        .char_indices()
        .find(|(_, character)| {
            !(character.is_ascii_digit() || *character == '.' || *character == ',')
        })
        .map(|(index, _)| number_start + index)
        .unwrap_or(trimmed.len());

    let quantity = format!("{}{}", sign, &trimmed[number_start..number_end]);
    let prefix_commodity = trimmed[..number_start].trim().trim_matches('-').trim();
    let suffix_commodity = trimmed[number_end..].trim();
    let commodity = if !prefix_commodity.is_empty() {
        prefix_commodity
    } else {
        suffix_commodity
    };

    (quantity, commodity.to_string())
}

/// Splits a posting into account and amount using hledger's common spacing convention.
fn split_posting_account_amount(value: &str) -> (&str, &str) {
    let mut whitespace_start = None;
    let mut whitespace_len = 0;

    for (index, character) in value.char_indices() {
        if character.is_whitespace() {
            if whitespace_start.is_none() {
                whitespace_start = Some(index);
            }
            whitespace_len += character.len_utf8();
            continue;
        }

        if let Some(start) = whitespace_start {
            if whitespace_len >= 2 {
                return (&value[..start], &value[index..]);
            }
        }
        whitespace_start = None;
        whitespace_len = 0;
    }

    (value, "")
}

/// Splits an inline hledger comment from a posting line.
fn split_inline_comment(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(';') {
        (&value[..index], value[index + 1..].trim())
    } else {
        (value, "")
    }
}

/// Splits the first token from a string.
fn split_first_token(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(char::is_whitespace) {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

/// Returns true when a line appears to start a ledger transaction.
fn is_transaction_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return false;
    }

    trimmed
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(false)
}

/// Finds a parsed transaction by id.
fn find_block(content: &str, id: &str) -> Result<TransactionBlock, String> {
    parse_transactions(content)
        .into_iter()
        .find(|transaction| transaction.id == id)
        .map(|transaction| TransactionBlock { transaction })
        .ok_or_else(|| format!("Transaction not found: {}", id))
}

/// Splits content into normalized lines for range replacement.
fn split_lines(content: &str) -> Vec<String> {
    content.lines().map(ToString::to_string).collect()
}

/// Replaces a one-based inclusive line range.
fn replace_line_range(
    lines: &[String],
    start_line: usize,
    end_line: usize,
    replacement: &str,
) -> String {
    let mut result = Vec::new();
    let start_index = start_line.saturating_sub(1);
    let end_index = end_line.min(lines.len());

    result.extend_from_slice(&lines[..start_index]);
    if !replacement.trim().is_empty() {
        result.extend(replacement.lines().map(ToString::to_string));
    }
    result.extend_from_slice(&lines[end_index..]);

    let mut content = result.join("\n");
    content.push('\n');
    content
}

/// Formats a transaction from structured form input.
fn format_transaction(input: &TransactionInput) -> String {
    let mut header = input.date.trim().to_string();
    if !input.status.trim().is_empty() {
        header.push(' ');
        header.push_str(input.status.trim());
    }
    if !input.code.trim().is_empty() {
        header.push(' ');
        header.push_str(input.code.trim());
    }
    if !input.description.trim().is_empty() {
        header.push(' ');
        header.push_str(input.description.trim());
    }

    let postings = input
        .postings
        .iter()
        .filter(|posting| !posting.account.trim().is_empty())
        .map(|posting| format_posting(posting))
        .collect::<Vec<_>>();

    std::iter::once(header)
        .chain(postings)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalizes a numeric quantity to two decimals when possible.
fn normalize_quantity(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.replace(',', ".");
    match normalized.parse::<f64>() {
        Ok(value) => format!("{:.2}", value),
        Err(_) => trimmed.to_string(),
    }
}

/// Formats a quantity and commodity into hledger amount syntax.
fn format_posting_amount(amount: &str, commodity: &str) -> String {
    let (quantity, parsed_commodity) = parse_posting_amount(amount);
    let selected_commodity = if parsed_commodity.trim().is_empty() {
        commodity.trim()
    } else {
        parsed_commodity.trim()
    };
    let quantity = normalize_quantity(&quantity);

    if quantity.is_empty() {
        return String::new();
    }
    if selected_commodity.is_empty() {
        return quantity;
    }

    let sign = if quantity.starts_with('-') { "-" } else { "" };
    let absolute_quantity = quantity.trim_start_matches('-');
    if selected_commodity
        .chars()
        .all(|character| character.is_alphabetic())
    {
        format!("{}{} {}", sign, absolute_quantity, selected_commodity)
    } else {
        format!("{}{}{}", sign, selected_commodity, absolute_quantity)
    }
}

/// Formats a posting, including an optional hledger inline comment.
fn format_posting(posting: &PostingInput) -> String {
    let amount = format_posting_amount(&posting.amount, &posting.commodity);
    let mut line = if amount.trim().is_empty() {
        format!("    {}", posting.account.trim())
    } else {
        format!("    {:<40} {}", posting.account.trim(), amount)
    };

    if !posting.comment.trim().is_empty() {
        line.push_str("  ; ");
        line.push_str(posting.comment.trim().trim_start_matches(';').trim());
    }

    line
}

/// Builds the transaction display fields consumed by the frontend.
fn summarize_transaction(postings: &[JournalPosting]) -> TransactionDisplay {
    let postings_with_amounts = postings
        .iter()
        .filter(|posting| !posting.amount.trim().is_empty())
        .collect::<Vec<_>>();
    let balancing_amount = postings_with_amounts
        .first()
        .map(|posting| format_posting_amount(&posting.amount, &posting.commodity))
        .unwrap_or_default();

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("expenses"))
    {
        let amount = if posting.amount.trim().is_empty() {
            balancing_amount
        } else {
            format_posting_amount(&posting.amount, &posting.commodity)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "expense"),
            kind: "expense".to_string(),
        };
    }

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("income"))
    {
        let amount = if posting.amount.trim().is_empty() {
            balancing_amount
        } else {
            format_posting_amount(&posting.amount, &posting.commodity)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "income"),
            kind: "income".to_string(),
        };
    }

    if let Some(posting) = postings_with_amounts.iter().find(|posting| {
        posting.account.to_lowercase().starts_with("assets")
            && parse_amount_value(&posting.amount) > 0.0
    }) {
        let amount = format_posting_amount(&posting.amount, &posting.commodity);
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "income"),
            kind: "income".to_string(),
        };
    }

    if let Some(posting) = postings_with_amounts
        .first()
        .copied()
        .or_else(|| postings.first())
    {
        let amount = format_posting_amount(&posting.amount, &posting.commodity);
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: if amount.is_empty() {
                "—".to_string()
            } else {
                amount
            },
            kind: if posting.amount.trim().is_empty() {
                "unknown".to_string()
            } else {
                "transfer".to_string()
            },
        };
    }

    TransactionDisplay {
        account: "—".to_string(),
        amount: "—".to_string(),
        kind: "unknown".to_string(),
    }
}

/// Parses a numeric value from an amount quantity.
fn parse_amount_value(amount: &str) -> f64 {
    amount
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .unwrap_or_default()
}

/// Formats the display amount sign according to the inferred transaction kind.
fn format_display_amount(amount: &str, kind: &str) -> String {
    if amount.trim().is_empty() {
        return "—".to_string();
    }

    let normalized = amount.replace('-', "");
    match kind {
        "income" => format!("+{}", normalized),
        "expense" => format!("-{}", normalized),
        _ => amount.to_string(),
    }
}

/// Builds dashboard-specific transaction groups.
fn build_dashboard_summary(transactions: &[JournalTransaction]) -> DashboardSummary {
    let monthly_transactions = transactions
        .iter()
        .filter(|transaction| is_in_current_month_to_date(transaction))
        .cloned()
        .collect::<Vec<_>>();
    let scheduled_transactions = transactions
        .iter()
        .filter(|transaction| is_scheduled_this_month(transaction))
        .cloned()
        .collect::<Vec<_>>();
    let active_accounts_count = monthly_transactions
        .iter()
        .chain(scheduled_transactions.iter())
        .flat_map(|transaction| transaction.postings.iter())
        .map(|posting| posting.account.to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .len();

    DashboardSummary {
        monthly_transactions,
        scheduled_transactions,
        active_accounts_count,
    }
}

/// Returns whether a transaction belongs to the current month up to today.
fn is_in_current_month_to_date(transaction: &JournalTransaction) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date <= today)
        .unwrap_or(false)
}

/// Returns whether a transaction is scheduled later in the current month.
fn is_scheduled_this_month(transaction: &JournalTransaction) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date > today)
        .unwrap_or(false)
}

fn parse_journal_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Sorts and removes duplicate values.
fn unique_sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort_by_key(|value| value.to_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    values
}

/// Extracts commodity-like tokens from posting amounts for display.
fn collect_commodities(transactions: &[JournalTransaction]) -> Vec<String> {
    let mut commodities = transactions
        .iter()
        .flat_map(|transaction| transaction.postings.iter())
        .filter_map(|posting| {
            if posting.commodity.trim().is_empty() {
                None
            } else {
                Some(posting.commodity.as_str())
            }
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    commodities.sort();
    commodities.dedup();
    commodities
}
