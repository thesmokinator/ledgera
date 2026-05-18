//! Application backend for Ledgera.
//!
//! The Rust layer owns journal access, conservative transaction edits, settings
//! persistence, and integration with the official hledger CLI.

use chrono::{Datelike, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fmt, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use tauri::{AppHandle, Manager};

static AMOUNT_STYLE: OnceLock<AmountStyle> = OnceLock::new();

/// Structured application error serialized as JSON over the Tauri bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl AppError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Serializes an AppError into a JSON string that the frontend can parse.
fn to_error_string(code: &str, message: impl Into<String>) -> String {
    let error = AppError::new(code, message);
    serde_json::to_string(&error)
        .unwrap_or_else(|_| format!(r#"{{"code":"{}","message":"Serialization failed"}}"#, code))
}

fn to_error_string_with_details(
    code: &str,
    message: impl Into<String>,
    details: impl Into<String>,
) -> String {
    let error = AppError::new(code, message).with_details(details);
    serde_json::to_string(&error)
        .unwrap_or_else(|_| format!(r#"{{"code":"{}","message":"Serialization failed"}}"#, code))
}

// ── Logging ──────────────────────────────────────────────────────────────

const LOG_RETENTION_DAYS: i64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry {
    ts: String,
    level: String,
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("ledgera.log.jsonl"))
}

fn log_event(app: &AppHandle, level: &str, code: &str, message: &str) {
    log_event_with_details(app, level, code, message, None);
}

fn log_event_with_details(
    app: &AppHandle,
    level: &str,
    code: &str,
    message: &str,
    details: Option<String>,
) {
    let path = match log_path(app) {
        Ok(p) => p,
        Err(_) => return,
    };
    let entry = LogEntry {
        ts: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        level: level.to_string(),
        code: code.to_string(),
        message: message.to_string(),
        details,
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let mut line = json;
        line.push('\n');
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
    }
}

fn cleanup_old_logs(app: &AppHandle) {
    let path = match log_path(app) {
        Ok(p) => p,
        Err(_) => return,
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let cutoff = Utc::now() - chrono::Duration::days(LOG_RETENTION_DAYS);
    let cutoff_str = cutoff.format("%Y-%m-%dT").to_string();
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| line.trim() >= cutoff_str.as_str())
        .collect();
    if kept.len() != content.lines().count() {
        let _ = fs::write(&path, kept.join("\n") + "\n");
    }
}

#[tauri::command]
fn get_logs(app: AppHandle) -> Result<Vec<LogEntry>, String> {
    let path = log_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| {
        to_error_string_with_details(
            "LOG_READ_FAILED",
            "Unable to read log file.",
            error.to_string(),
        )
    })?;
    let mut entries: Vec<LogEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .collect();
    entries.reverse();
    Ok(entries)
}

#[tauri::command]
fn clear_logs(app: AppHandle) -> Result<(), String> {
    let path = log_path(&app)?;
    fs::write(&path, "").map_err(|error| {
        to_error_string_with_details(
            "LOG_WRITE_FAILED",
            "Unable to clear log file.",
            error.to_string(),
        )
    })
}

// ── Holdings & Prices ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Holding {
    commodity: String,
    quantity: f64,
    account: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PriceInfo {
    price: f64,
    currency: String,
    formatted: String,
}

#[tauri::command]
fn get_holdings(app: AppHandle) -> Result<Vec<Holding>, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let files = load_journal_files(&journal_path)?;
    let transactions = load_transactions_from_journal_via_files(&files)?;

    let default_commodity = settings.default_commodity.trim().to_lowercase();
    let mut quantities: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut accounts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut original_commodity: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for tx in &transactions {
        for posting in &tx.postings {
            let commodity_raw = posting.commodity.trim();
            // Strip @ price notation (e.g. "XEON @ €148.70" → "XEON")
            let commodity_clean = if let Some(at_pos) = commodity_raw.find(" @ ") {
                commodity_raw[..at_pos].trim()
            } else {
                commodity_raw
            };
            let commodity_lower = commodity_clean.to_lowercase();
            if commodity_lower.is_empty() || commodity_lower == default_commodity {
                continue;
            }
            let amount = posting.amount.trim().replace(',', ".");
            if let Ok(qty) = amount.parse::<f64>() {
                *quantities.entry(commodity_lower.clone()).or_default() += qty;
                accounts
                    .entry(commodity_lower.clone())
                    .or_insert_with(|| posting.account.trim().to_string());
                original_commodity
                    .entry(commodity_lower.clone())
                    .or_insert_with(|| commodity_clean.to_string());
            }
        }
    }

    let include_set: std::collections::HashSet<String> = settings
        .include_investments
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut result: Vec<Holding> = Vec::new();
    for (commodity_lower, quantity) in quantities {
        if quantity.abs() <= 0.0001 {
            continue;
        }
        if !include_set.is_empty() {
            let acct = accounts.get(&commodity_lower);
            if acct.map_or(true, |a| !include_set.contains(a.as_str())) {
                continue;
            }
        }
        let account = accounts.remove(&commodity_lower).unwrap_or_default();
        let commodity = original_commodity
            .remove(&commodity_lower)
            .unwrap_or(commodity_lower);
        result.push(Holding {
            account,
            commodity,
            quantity,
        });
    }
    result.sort_by(|a, b| a.commodity.cmp(&b.commodity));
    Ok(result)
}

#[tauri::command]
async fn get_investments(app: AppHandle) -> Result<Vec<Balance>, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let executable = hledger_executable(&settings);

    let include_accounts: Vec<&str> = settings
        .include_investments
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if include_accounts.is_empty() {
        return Ok(Vec::new());
    }

    let mut cmd = Command::new(&executable);
    cmd.arg("-f").arg(&journal_path).arg("balance");
    for acct in &include_accounts {
        cmd.arg(acct);
    }
    cmd.arg("-O").arg("json");

    let output =
        tauri::async_runtime::spawn_blocking(move || cmd.output().map_err(|e| e.to_string()))
            .await
            .map_err(|error| {
                to_error_string_with_details(
                    "HLEDGER_BALANCE_FAILED",
                    "Unable to run hledger balance for investments.",
                    error.to_string(),
                )
            })??;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_balance_output(&app, &stdout, &settings, false)
}

fn parse_balance_output(
    _app: &AppHandle,
    stdout: &str,
    settings: &AppSettings,
    apply_exclude: bool,
) -> Result<Vec<Balance>, String> {
    let raw: Vec<serde_json::Value> = serde_json::from_str(stdout).map_err(|error| {
        to_error_string_with_details(
            "HLEDGER_BALANCE_PARSE_FAILED",
            "Unable to parse hledger balance output.",
            error.to_string(),
        )
    })?;

    let rows = raw.first().and_then(|v| v.as_array()).ok_or_else(|| {
        to_error_string(
            "HLEDGER_BALANCE_PARSE_FAILED",
            "Unexpected hledger balance JSON structure.",
        )
    })?;

    let exclude_set: std::collections::HashSet<&str> = settings
        .exclude_balances
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Always log what we're filtering
    eprintln!(
        "BALANCE_FILTER exclude: '{}' ({}), include: '{}' ({})",
        settings.exclude_balances,
        exclude_set.len(),
        settings.include_investments,
        settings
            .include_investments
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count()
    );

    let mut result: Vec<Balance> = Vec::new();
    for row in rows {
        let arr = match row.as_array() {
            Some(a) if a.len() >= 4 => a,
            _ => continue,
        };
        let account = arr[0].as_str().unwrap_or("").to_string();
        if apply_exclude && exclude_set.contains(account.as_str()) {
            continue;
        }
        let (amount, commodity, formatted) =
            if let Some(bal) = arr[3].as_array().and_then(|a| a.first()) {
                let comm = bal["acommodity"].as_str().unwrap_or("").to_string();
                if AMOUNT_STYLE.get().is_none() {
                    let style = AmountStyle::from_hledger_json(bal);
                    let _ = AMOUNT_STYLE.set(style);
                }
                let qty = bal["aquantity"]
                    .get("floatingPoint")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let fmt = format_hledger_amount(qty, bal);
                (qty, comm, fmt)
            } else {
                (0.0, String::new(), String::new())
            };
        result.push(Balance {
            account,
            amount,
            commodity,
            formatted,
        });
    }
    Ok(result)
}

/// Fetches current prices from Yahoo Finance for a list of symbols.
#[tauri::command]
async fn fetch_prices(
    app: AppHandle,
    symbols: Vec<String>,
) -> Result<std::collections::HashMap<String, PriceInfo>, String> {
    let settings = read_settings(&app)?;
    log_event(
        &app,
        "info",
        "PRICE_FETCH_START",
        &format!("Fetching prices for {} symbols", symbols.len()),
    );

    if !settings.fetch_prices {
        return Err(to_error_string(
            "PRICES_DISABLED",
            "Market price fetching is disabled in Settings.",
        ));
    }

    let mut prices = std::collections::HashMap::new();

    // Parse commodity symbols mapping (format: "VWCE=VWCE.DE\nXEON=XEON.DE")
    let mapping: std::collections::HashMap<String, String> = settings
        .commodity_symbols
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    for symbol in &symbols {
        let yahoo_symbol = mapping.get(symbol).unwrap_or(symbol);
        log_event(
            &app,
            "info",
            "PRICE_FETCH_TRY",
            &format!("Trying {} → {}", symbol, yahoo_symbol),
        );

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}",
            yahoo_symbol
        );
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            )
            .header("Accept", "application/json")
            .send()
            .await;
        match response {
            Ok(response) => {
                log_event(
                    &app,
                    "info",
                    "PRICE_FETCH_RESPONSE",
                    &format!("Got response for {}", yahoo_symbol),
                );
                let body_text = response.text().await.unwrap_or_default();
                match serde_json::from_str::<serde_json::Value>(&body_text) {
                    Ok(json) => {
                        let meta = &json["chart"]["result"][0]["meta"];
                        if let (Some(price), Some(currency)) = (
                            meta["regularMarketPrice"].as_f64(),
                            meta["currency"].as_str(),
                        ) {
                            let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
                                static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
                                DEFAULT.get_or_init(AmountStyle::default)
                            });
                            let formatted = style.format(price);
                            prices.insert(
                                symbol.clone(),
                                PriceInfo {
                                    price,
                                    currency: currency.to_string(),
                                    formatted,
                                },
                            );
                            log_event(
                                &app,
                                "info",
                                "PRICE_FETCHED",
                                &format!("{} = {} {}", symbol, price, currency),
                            );
                        } else {
                            log_event_with_details(
                                &app,
                                "warn",
                                "PRICE_PARSE_FAILED",
                                &format!("Could not parse price/currency for {}", symbol),
                                Some(body_text),
                            );
                        }
                    }
                    Err(e) => {
                        log_event_with_details(
                            &app,
                            "warn",
                            "PRICE_JSON_FAILED",
                            &format!("Response not valid JSON for {}", symbol),
                            Some(format!("Parse error: {}\nBody: {}", e, body_text)),
                        );
                    }
                }
            }
            Err(error) => {
                log_event_with_details(
                    &app,
                    "error",
                    "PRICE_FETCH_FAILED",
                    &format!("HTTP request failed for {}", symbol),
                    Some(error.to_string()),
                );
            }
        }
    }
    Ok(prices)
}

/// Display style for amounts (decimal mark, digit grouping, precision).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AmountStyle {
    decimal_mark: String,
    digit_separator: String,
    digit_groups: Vec<usize>,
    precision: usize,
}

impl Default for AmountStyle {
    fn default() -> Self {
        Self {
            decimal_mark: ".".to_string(),
            digit_separator: ",".to_string(),
            digit_groups: vec![3],
            precision: 2,
        }
    }
}

impl AmountStyle {
    fn from_hledger_json(bal: &serde_json::Value) -> Self {
        let style = &bal["astyle"];
        let decimal_mark = style["asdecimalmark"].as_str().unwrap_or(".").to_string();
        let digit_groups: Vec<usize> = style["asdigitgroups"]
            .as_array()
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_default();
        let digit_separator = style["asdigitgroups"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let precision = style["asprecision"].as_u64().unwrap_or(2) as usize;
        Self {
            decimal_mark,
            digit_separator,
            digit_groups,
            precision,
        }
    }

    fn format(&self, amount: f64) -> String {
        let abs = amount.abs();
        let sign = if amount < 0.0 { "-" } else { "" };
        let formatted_num = format!("{:.*}", self.precision, abs);
        let parts: Vec<&str> = formatted_num.split('.').collect();
        let int_part = parts[0];
        let frac_part = parts.get(1).unwrap_or(&"");

        let grouped_int = if self.digit_groups.is_empty() || self.digit_separator.is_empty() {
            int_part.to_string()
        } else {
            let mut result = String::new();
            let mut remaining = int_part.len();
            let mut group_iter = self.digit_groups.iter().cycle();
            let mut first = true;
            while remaining > 0 {
                if !first {
                    result.insert_str(0, &self.digit_separator);
                }
                first = false;
                let group_size = group_iter.next().unwrap_or(&3);
                let take = (*group_size).min(remaining);
                let start = remaining - take;
                result.insert_str(0, &int_part[start..remaining]);
                remaining = start;
            }
            result
        };

        let decimal_part = if frac_part.is_empty() {
            String::new()
        } else {
            format!("{}{}", self.decimal_mark, frac_part)
        };

        format!("{}{}{}", sign, grouped_int, decimal_part)
    }
}

/// Returns account balances from hledger as a hierarchical tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Balance {
    account: String,
    amount: f64,
    commodity: String,
    formatted: String,
}

/// Formats a number according to hledger display style (decimal mark, digit groups).
fn format_hledger_amount(amount: f64, bal: &serde_json::Value) -> String {
    AmountStyle::from_hledger_json(bal).format(amount)
}

/// Formats a number using the journal's display style.
#[tauri::command]
fn format_number(value: f64) -> String {
    let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
        static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
        DEFAULT.get_or_init(AmountStyle::default)
    });
    style.format(value)
}

#[tauri::command]
async fn get_balances(app: AppHandle) -> Result<Vec<Balance>, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let executable = hledger_executable(&settings);

    log_event(&app, "info", "BALANCE_START", "Running hledger balance");

    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new(&executable)
            .arg("-f")
            .arg(&journal_path)
            .arg("balance")
            .arg("-O")
            .arg("json")
            .output()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| {
        to_error_string_with_details(
            "HLEDGER_BALANCE_FAILED",
            "Unable to run hledger balance.",
            error.to_string(),
        )
    })??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        log_event_with_details(
            &app,
            "error",
            "BALANCE_FAILED",
            "hledger balance failed",
            Some(stderr.clone()),
        );
        return Err(to_error_string_with_details(
            "HLEDGER_BALANCE_FAILED",
            "hledger balance command failed.",
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    log_event_with_details(
        &app,
        "info",
        "BALANCE_RAW",
        "hledger balance output",
        Some(stdout.clone()),
    );

    parse_balance_output(&app, &stdout, &settings, true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    journal_path: String,
    hledger_path: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    power_user: bool,
    #[serde(default)]
    default_commodity: String,
    #[serde(default)]
    fetch_prices: bool,
    #[serde(default)]
    commodity_symbols: String,
    #[serde(default)]
    exclude_balances: String,
    #[serde(default)]
    include_investments: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HledgerStatus {
    available: bool,
    version: String,
    message: String,
    resolved_path: String,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalSummary {
    path: String,
    transactions: Vec<JournalTransaction>,
    commodities: Vec<String>,
    file_count: usize,
    total_size_bytes: u64,
    amount_style: AmountStyle,
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
    default_commodity: String,
    default_cash_account: String,
    default_expense_account: String,
    default_income_account: String,
    default_transfer_account: String,
    default_investment_account: String,
    default_investment_commodity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalTransaction {
    id: String,
    source_file: String,
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
    formatted: String,
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
    unit_price: String,
    #[serde(default)]
    comment: String,
}

#[derive(Debug, Clone)]
struct TransactionBlock {
    transaction: JournalTransaction,
}

#[derive(Debug, Clone)]
struct JournalFile {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RoutingStrategy {
    Glob(Vec<String>),
    Flat(Vec<String>),
    Fallback,
}

#[derive(Debug, Clone, Default)]
struct JournalProfile {
    default_cash_account: String,
    default_expense_account: String,
    default_income_account: String,
    default_transfer_account: String,
    default_investment_account: String,
    default_investment_commodity: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            journal_path: String::new(),
            hledger_path: String::new(),
            theme: default_theme(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: String::new(),
            exclude_balances: String::new(),
            include_investments: String::new(),
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
    log_event(
        &app,
        "info",
        "SETTINGS_SAVE",
        &format!("Saving exclude_balances: '{}'", settings.exclude_balances),
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            to_error_string_with_details(
                "SETTINGS_SAVE_FAILED",
                "Unable to create settings directory.",
                error.to_string(),
            )
        })?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| {
        to_error_string_with_details(
            "SETTINGS_SAVE_FAILED",
            "Unable to encode settings.",
            error.to_string(),
        )
    })?;
    fs::write(&path, content).map_err(|error| {
        to_error_string_with_details(
            "SETTINGS_SAVE_FAILED",
            "Unable to write settings file.",
            error.to_string(),
        )
    })?;
    Ok(settings)
}

/// Checks whether the configured hledger executable can be invoked.
#[tauri::command]
fn check_hledger(app: AppHandle) -> Result<HledgerStatus, String> {
    let settings = read_settings(&app)?;
    let (executable, source) = hledger_executable_with_source(&settings);
    let output = Command::new(&executable).arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(HledgerStatus {
                available: true,
                version: version.clone(),
                message: version,
                resolved_path: executable,
                source,
            })
        }
        Ok(output) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            resolved_path: executable,
            source,
        }),
        Err(error) => Ok(HledgerStatus {
            available: false,
            version: String::new(),
            message: error.to_string(),
            resolved_path: executable,
            source,
        }),
    }
}

/// Reads distinct values that can be reused while editing transactions.
#[tauri::command]
fn get_autocomplete_suggestions(app: AppHandle) -> Result<AutocompleteSuggestions, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let transactions = load_transactions_from_journal(&journal_path)?;

    let profile = build_journal_profile(&transactions, settings.default_commodity.trim());

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
        default_commodity: settings.default_commodity.trim().to_string(),
        default_cash_account: profile.default_cash_account,
        default_expense_account: profile.default_expense_account,
        default_income_account: profile.default_income_account,
        default_transfer_account: profile.default_transfer_account,
        default_investment_account: profile.default_investment_account,
        default_investment_commodity: profile.default_investment_commodity,
    })
}

/// Reads the configured journal and returns parsed transaction blocks.
#[tauri::command]
fn list_transactions(app: AppHandle) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    read_journal_summary(&journal_path)
}

/// Appends a new transaction using the existing journal style where possible.
#[tauri::command]
fn create_transaction(app: AppHandle, input: TransactionInput) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = create_transaction_for_settings(&settings, &input);
    match &result {
        Ok(_) => log_event(&app, "info", "TRANSACTION_CREATED", &input.description),
        Err(e) => log_event_with_details(
            &app,
            "error",
            "TRANSACTION_CREATE_FAILED",
            "Failed to create transaction",
            Some(e.clone()),
        ),
    }
    result
}

/// Replaces an existing transaction block by id.
#[tauri::command]
fn update_transaction(
    app: AppHandle,
    id: String,
    input: TransactionInput,
) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = update_transaction_for_settings(&settings, &id, &input);
    match &result {
        Ok(_) => log_event(&app, "info", "TRANSACTION_UPDATED", &input.description),
        Err(e) => log_event_with_details(
            &app,
            "error",
            "TRANSACTION_UPDATE_FAILED",
            "Failed to update transaction",
            Some(e.clone()),
        ),
    }
    result
}

/// Removes an existing transaction block by id.
#[tauri::command]
fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
    let settings = read_settings(&app)?;
    let result = delete_transaction_for_settings(&settings, &id);
    match &result {
        Ok(_) => log_event(&app, "info", "TRANSACTION_DELETED", &id),
        Err(e) => log_event_with_details(
            &app,
            "error",
            "TRANSACTION_DELETE_FAILED",
            "Failed to delete transaction",
            Some(e.clone()),
        ),
    }
    result
}

/// Starts the Tauri application and registers backend commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            cleanup_old_logs(&app.handle());

            let win_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                    .title("Ledgera")
                    .inner_size(1400.0, 918.0)
                    .min_inner_size(1080.0, 720.0);

            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(tauri::TitleBarStyle::Transparent);

            let _window = win_builder.build().unwrap();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            update_app_settings,
            check_hledger,
            get_autocomplete_suggestions,
            list_transactions,
            create_transaction,
            update_transaction,
            delete_transaction,
            get_logs,
            clear_logs,
            get_holdings,
            get_investments,
            fetch_prices,
            get_balances,
            format_number,
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

    let content = fs::read_to_string(&path).map_err(|error| {
        to_error_string_with_details(
            "SETTINGS_READ_FAILED",
            "Unable to read settings file.",
            error.to_string(),
        )
    })?;
    serde_json::from_str(&content).map_err(|error| {
        to_error_string_with_details(
            "SETTINGS_READ_FAILED",
            "Settings file is corrupted.",
            error.to_string(),
        )
    })
}

/// Resolves the hledger executable from settings or common macOS/user-shell locations.
fn hledger_executable(settings: &AppSettings) -> String {
    hledger_executable_with_source(settings).0
}

fn hledger_executable_with_source(settings: &AppSettings) -> (String, String) {
    let configured = settings.hledger_path.trim();
    if !configured.is_empty() {
        return (configured.to_string(), "configured".to_string());
    }

    if let Some(detected) = find_hledger_executable() {
        return (detected, "detected".to_string());
    }

    ("hledger".to_string(), "fallback".to_string())
}

/// Finds hledger in common installation folders and in the user's login shell PATH.
fn find_hledger_executable() -> Option<String> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/hledger"),
        PathBuf::from("/usr/local/bin/hledger"),
        PathBuf::from("/usr/bin/hledger"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/bin/hledger"));
        candidates.push(home.join(".cabal/bin/hledger"));
    }

    candidates
        .into_iter()
        .chain(
            login_shell_path_dirs()
                .into_iter()
                .map(|path| path.join("hledger")),
        )
        .find(|path| is_executable_file(path))
        .map(|path| path.to_string_lossy().to_string())
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
        && Command::new(path)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
}

fn login_shell_path_dirs() -> Vec<PathBuf> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    [["-li", "-c", "echo $PATH"], ["-l", "-c", "echo $PATH"]]
        .into_iter()
        .find_map(|args| {
            Command::new(&shell)
                .args(args)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .and_then(|output| {
            output
                .lines()
                .rev()
                .map(str::trim)
                .find(|line| line.contains('/') && !line.is_empty())
                .map(|line| line.split(':').map(PathBuf::from).collect())
        })
        .unwrap_or_default()
}

/// Resolves the configured journal path.
fn require_journal_path(settings: &AppSettings) -> Result<PathBuf, String> {
    if settings.journal_path.trim().is_empty() {
        return Err(to_error_string(
            "JOURNAL_NOT_CONFIGURED",
            "Configure a journal path in Settings before loading transactions.",
        ));
    }

    let path = PathBuf::from(settings.journal_path.trim());
    if !path.exists() {
        return Err(to_error_string_with_details(
            "JOURNAL_NOT_FOUND",
            "Journal file does not exist.",
            format!("Expected at: {}", path.display()),
        ));
    }
    Ok(path)
}

/// Parses the display style from commodity/format directives in journal files.
fn parse_amount_style(files: &[JournalFile], _default_commodity: &str) -> AmountStyle {
    for file in files {
        let mut in_commodity = false;
        for line in file.content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("commodity ") {
                in_commodity = true;
                continue;
            }
            if in_commodity && trimmed.starts_with("format ") {
                let fmt = trimmed.strip_prefix("format ").unwrap_or("").trim();
                // format looks like: €1.000,00 or $1,234.56
                // Extract decimal mark (last non-digit char before the cents)
                if let Some(style) = parse_format_directive(fmt) {
                    return style;
                }
                in_commodity = false;
            }
            if in_commodity && !line.starts_with(' ') && !trimmed.is_empty() {
                in_commodity = false;
            }
        }
    }
    AmountStyle::default()
}

fn parse_format_directive(fmt: &str) -> Option<AmountStyle> {
    // fmt looks like: "€1.000,00" or "$1,234.56" or "1,234.56"
    // Strip currency prefix (anything non-digit, non-separator at start)
    let num_part = fmt.trim_start_matches(|c: char| !c.is_ascii_digit() && c != '-');
    if num_part.is_empty() {
        return None;
    }

    // Find decimal mark: last non-digit character followed by digits at end
    let chars: Vec<char> = num_part.chars().collect();
    let len = chars.len();

    // Count trailing digits (decimal places)
    let mut trailing_digits = 0;
    for i in (0..len).rev() {
        if chars[i].is_ascii_digit() {
            trailing_digits += 1;
        } else {
            break;
        }
    }

    if trailing_digits == 0 || trailing_digits == len {
        // No decimal part
        let separator = chars
            .iter()
            .rev()
            .skip(trailing_digits)
            .find(|c| !c.is_ascii_digit())
            .map(|c| c.to_string())
            .unwrap_or_default();
        return Some(AmountStyle {
            decimal_mark: ".".to_string(),
            digit_separator: separator.clone(),
            digit_groups: if separator.is_empty() {
                vec![]
            } else {
                vec![3]
            },
            precision: 0,
        });
    }

    let decimal_mark = chars[len - trailing_digits - 1].to_string();
    let int_part: String = chars[..len - trailing_digits - 1].iter().collect();

    // Find digit group separator from the integer part
    let separator = int_part
        .chars()
        .rev()
        .find(|c| !c.is_ascii_digit())
        .map(|c| c.to_string())
        .unwrap_or_default();

    Some(AmountStyle {
        decimal_mark,
        digit_separator: separator.clone(),
        digit_groups: if separator.is_empty() {
            vec![]
        } else {
            vec![3]
        },
        precision: trailing_digits,
    })
}

fn read_journal_summary(journal_path: &Path) -> Result<JournalSummary, String> {
    let files = load_journal_files(journal_path)?;
    let file_count = files.len();
    let total_size_bytes: u64 = files.iter().map(|f| f.content.len() as u64).sum();

    let amount_style = parse_amount_style(&files, "€");
    let _ = AMOUNT_STYLE.set(amount_style.clone());

    let transactions = load_transactions_from_journal_via_files(&files)?;
    let commodities = collect_commodities(&transactions);
    let dashboard = build_dashboard_summary(&transactions);

    Ok(JournalSummary {
        path: journal_path.to_string_lossy().to_string(),
        transactions,
        commodities,
        file_count,
        total_size_bytes,
        amount_style,
        dashboard,
    })
}

fn load_transactions_from_journal(journal_path: &Path) -> Result<Vec<JournalTransaction>, String> {
    let files = load_journal_files(journal_path)?;
    load_transactions_from_journal_via_files(&files)
}

fn load_transactions_from_journal_via_files(
    files: &[JournalFile],
) -> Result<Vec<JournalTransaction>, String> {
    let mut transactions: Vec<JournalTransaction> = files
        .iter()
        .flat_map(|file| parse_transactions(&file.content, &file.path))
        .collect();
    transactions.reverse();
    transactions.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(transactions)
}

fn load_journal_files(journal_path: &Path) -> Result<Vec<JournalFile>, String> {
    let mut files = Vec::new();
    let mut visited = HashSet::new();
    load_journal_file_recursive(journal_path, &mut visited, &mut files)?;
    Ok(files)
}

fn load_journal_file_recursive(
    journal_path: &Path,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<JournalFile>,
) -> Result<(), String> {
    let canonical_path =
        fs::canonicalize(journal_path).unwrap_or_else(|_| journal_path.to_path_buf());
    if !visited.insert(canonical_path.clone()) {
        return Ok(());
    }

    let content = fs::read_to_string(&canonical_path).map_err(|error| {
        to_error_string_with_details(
            "JOURNAL_READ_FAILED",
            "Unable to read journal file.",
            format!("{}: {}", canonical_path.display(), error),
        )
    })?;
    let include_paths = find_include_paths(&canonical_path, &content)?;
    files.push(JournalFile {
        path: canonical_path,
        content,
    });

    for include_path in include_paths {
        load_journal_file_recursive(&include_path, visited, files)?;
    }

    Ok(())
}

fn find_include_paths(journal_path: &Path, content: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in content.lines() {
        if let Some(include) = parse_include_directive(line) {
            paths.extend(resolve_include_paths(journal_path, &include)?);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn parse_include_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return None;
    }

    let include = trimmed.strip_prefix("include")?.trim_start();
    if include.is_empty() {
        return None;
    }

    let include = split_inline_comment(include).0.trim();
    Some(include.trim_matches('"').to_string())
}

fn resolve_include_paths(journal_path: &Path, include: &str) -> Result<Vec<PathBuf>, String> {
    let base_dir = journal_path.parent().unwrap_or_else(|| Path::new("."));
    let include_path = PathBuf::from(include);
    let absolute_pattern = if include_path.is_absolute() {
        include_path
    } else {
        base_dir.join(include_path)
    };

    if !include.contains('*') {
        return if absolute_pattern.exists() {
            Ok(vec![absolute_pattern])
        } else {
            Err(to_error_string_with_details(
                "JOURNAL_INCLUDE_MISSING",
                "Included journal file does not exist.",
                format!("Expected at: {}", absolute_pattern.display()),
            ))
        };
    }

    expand_simple_glob(&absolute_pattern)
}

fn expand_simple_glob(pattern: &Path) -> Result<Vec<PathBuf>, String> {
    let parent = pattern.parent().unwrap_or_else(|| Path::new("."));
    let Some(file_pattern) = pattern.file_name().and_then(|name| name.to_str()) else {
        return Ok(Vec::new());
    };

    if !parent.exists() {
        return Ok(Vec::new());
    }

    let mut matches = fs::read_dir(parent)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| wildcard_matches(file_pattern, name))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == value;
    }

    let mut remainder = value;
    if let Some(first) = parts.first() {
        if !first.is_empty() {
            let Some(stripped) = remainder.strip_prefix(first) else {
                return false;
            };
            remainder = stripped;
        }
    }

    for part in parts.iter().skip(1).take(parts.len().saturating_sub(2)) {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remainder.find(part) else {
            return false;
        };
        remainder = &remainder[index + part.len()..];
    }

    if let Some(last) = parts.last() {
        last.is_empty() || remainder.ends_with(last)
    } else {
        true
    }
}

fn create_transaction_for_settings(
    settings: &AppSettings,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    append_transaction_routed(settings, &journal_path, input)?;
    read_journal_summary(&journal_path)
}

fn update_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    let replacement = format_transaction(input);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            &replacement,
        ))
    })?;
    read_journal_summary(&journal_path)
}

fn delete_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            "",
        ))
    })?;
    read_journal_summary(&journal_path)
}

fn mutate_existing_file<F>(
    settings: &AppSettings,
    main_journal: &Path,
    source_path: &Path,
    mutate: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let original = fs::read_to_string(source_path).map_err(|error| {
        to_error_string_with_details(
            "JOURNAL_READ_FAILED",
            "Unable to read source journal file.",
            format!("{}: {}", source_path.display(), error),
        )
    })?;
    let updated = mutate(&original)?;

    fs::write(source_path, &updated).map_err(|error| {
        to_error_string_with_details(
            "JOURNAL_WRITE_FAILED",
            "Unable to write journal file.",
            error.to_string(),
        )
    })?;
    if let Err(error) = validate_journal(settings, main_journal) {
        fs::write(source_path, original).map_err(|rollback_error| {
            to_error_string_with_details(
                "JOURNAL_WRITE_FAILED",
                "Journal file may be corrupted — rollback failed.",
                rollback_error.to_string(),
            )
        })?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_routed(
    settings: &AppSettings,
    main_journal: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    let content = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    match detect_routing_strategy(&content) {
        RoutingStrategy::Fallback => {
            append_to_existing_file(settings, main_journal, main_journal, input)
        }
        RoutingStrategy::Flat(includes) => {
            let target_include = flat_target_include(&includes, input);
            let target_file = resolve_relative_to_main(main_journal, &target_include);
            if target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else {
                append_to_new_flat_subjournal(
                    settings,
                    main_journal,
                    &target_file,
                    &target_include,
                    input,
                )
            }
        }
        RoutingStrategy::Glob(includes) => {
            let (target_file, target_include) = glob_target_path(main_journal, &includes, input);
            if target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else if glob_include_matches_year(&target_include, &includes) {
                append_to_new_glob_subjournal(settings, main_journal, &target_file, input)
            } else {
                append_to_new_glob_year(
                    settings,
                    main_journal,
                    &target_file,
                    &target_include,
                    input,
                )
            }
        }
    }
}

fn append_to_existing_file(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    mutate_existing_file(settings, main_journal, target_file, |content| {
        Ok(append_transaction_text(content, input))
    })
}

fn append_to_new_flat_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    target_name: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let updated_main = insert_include_sorted(&original_main, target_name);

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_year(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    target_include: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let year_dir = target_file
        .parent()
        .ok_or_else(|| "Unable to resolve target journal directory.".to_string())?;
    let year_dir_created = !year_dir.exists();
    let updated_main = insert_glob_include_sorted(&original_main, target_include);

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::create_dir_all(year_dir).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        if year_dir_created {
            let _ = fs::remove_dir(year_dir);
        }
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_text(content: &str, input: &TransactionInput) -> String {
    let mut updated = content.trim_end_matches(['\r', '\n']).to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&format_transaction(input));
    updated.push('\n');
    updated
}

fn detect_routing_strategy(content: &str) -> RoutingStrategy {
    let glob_includes = content
        .lines()
        .filter_map(parse_glob_include)
        .collect::<Vec<_>>();
    if !glob_includes.is_empty() {
        return RoutingStrategy::Glob(glob_includes);
    }

    let flat_files = content
        .lines()
        .filter_map(parse_flat_include_filename)
        .collect::<Vec<_>>();
    if !flat_files.is_empty() {
        return RoutingStrategy::Flat(flat_files);
    }

    RoutingStrategy::Fallback
}

fn parse_flat_include_filename(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    let file_name = Path::new(&include).file_name()?.to_str()?;
    if file_name.len() == "YYYY-MM.journal".len()
        && file_name.ends_with(".journal")
        && file_name
            .chars()
            .take(4)
            .all(|character| character.is_ascii_digit())
        && file_name.chars().nth(4) == Some('-')
        && file_name
            .chars()
            .skip(5)
            .take(2)
            .all(|character| character.is_ascii_digit())
    {
        Some(include)
    } else {
        None
    }
}

fn parse_glob_include(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    if include.ends_with("/*.journal") && glob_include_year(&include).is_some() {
        Some(include)
    } else {
        None
    }
}

fn glob_include_year(include: &str) -> Option<String> {
    let mut parts = include.split('/').collect::<Vec<_>>();
    if parts.pop()? != "*.journal" {
        return None;
    }
    parts
        .into_iter()
        .rev()
        .find(|part| part.len() == 4 && part.chars().all(|character| character.is_ascii_digit()))
        .map(ToString::to_string)
}

fn target_subjournal_name(input: &TransactionInput) -> String {
    format!("{}.journal", input.date.chars().take(7).collect::<String>())
}

fn flat_target_include(includes: &[String], input: &TransactionInput) -> String {
    let target_name = target_subjournal_name(input);
    if let Some(existing) = includes.iter().find(|include| {
        Path::new(include)
            .file_name()
            .and_then(|name| name.to_str())
            == Some(target_name.as_str())
    }) {
        return existing.clone();
    }

    let target_year = input.date.chars().take(4).collect::<String>();
    if let Some(similar_year_include) = includes.iter().find(|include| {
        Path::new(include)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with(&target_year))
            .unwrap_or(false)
    }) {
        return Path::new(similar_year_include)
            .parent()
            .map(|parent| parent.join(&target_name).to_string_lossy().to_string())
            .unwrap_or(target_name);
    }

    target_name
}

fn resolve_relative_to_main(main_journal: &Path, include: &str) -> PathBuf {
    let include_path = PathBuf::from(include);
    if include_path.is_absolute() {
        include_path
    } else {
        main_journal
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(include_path)
    }
}

fn glob_target_path(
    main_journal: &Path,
    includes: &[String],
    input: &TransactionInput,
) -> (PathBuf, String) {
    let year = input.date.chars().take(4).collect::<String>();
    let month = input.date.chars().skip(5).take(2).collect::<String>();
    let target_include = glob_target_include(includes, &year);
    let target_file = resolve_relative_to_main(
        main_journal,
        &target_include.replace("*.journal", &format!("{}.journal", month)),
    );
    (target_file, target_include)
}

fn glob_target_include(includes: &[String], year: &str) -> String {
    if let Some(existing) = includes
        .iter()
        .find(|include| glob_include_year(include).as_deref() == Some(year))
    {
        return existing.clone();
    }

    if let Some(first) = includes.first() {
        let mut parts = first.split('/').collect::<Vec<_>>();
        let _ = parts.pop();
        if let Some(index) = parts.iter().rposition(|part| {
            part.len() == 4 && part.chars().all(|character| character.is_ascii_digit())
        }) {
            parts[index] = year;
            return format!("{}/*.journal", parts.join("/"));
        }
    }

    format!("{}/*.journal", year)
}

fn glob_include_matches_year(target_include: &str, includes: &[String]) -> bool {
    let target_year = glob_include_year(target_include);
    includes
        .iter()
        .any(|include| glob_include_year(include) == target_year)
}

fn insert_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, parse_flat_include_filename)
}

fn insert_glob_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, parse_glob_include)
}

fn insert_sorted_include<F>(content: &str, new_include: &str, parser: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
    let include_positions = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| parser(line).map(|include| (index, include)))
        .collect::<Vec<_>>();
    let new_line = format!("include {}", new_include);

    if include_positions.is_empty() {
        lines.push(new_line);
    } else {
        let insert_index = include_positions
            .iter()
            .find(|(_, include)| new_include < include.as_str())
            .map(|(index, _)| *index)
            .unwrap_or_else(|| {
                include_positions
                    .last()
                    .map(|(index, _)| index + 1)
                    .unwrap_or(lines.len())
            });
        lines.insert(insert_index, new_line);
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
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
        Ok(output) => Err(to_error_string_with_details(
            "HLEDGER_CHECK_FAILED",
            "hledger check failed. The journal may contain syntax errors.",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
        Err(error) => Err(to_error_string_with_details(
            "HLEDGER_CHECK_FAILED",
            "Unable to run hledger check.",
            error.to_string(),
        )),
    }
}

/// Parses transaction blocks without attempting to reinterpret ledger semantics.
fn parse_transactions(content: &str, source_path: &Path) -> Vec<JournalTransaction> {
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
        if let Some(transaction) = parse_transaction_block(source_path, start_line, end_index, &raw)
        {
            transactions.push(transaction);
        }
        index = end_index;
    }

    transactions
}

/// Parses one transaction block.
fn parse_transaction_block(
    source_path: &Path,
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

    let source_file = source_path.to_string_lossy().to_string();
    Some(JournalTransaction {
        id: format!("{}:{}", source_file, start_line),
        source_file,
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
fn find_block(journal_path: &Path, id: &str) -> Result<TransactionBlock, String> {
    load_transactions_from_journal(journal_path)?
        .into_iter()
        .find(|transaction| transaction.id == id)
        .map(|transaction| TransactionBlock { transaction })
        .ok_or_else(|| {
            to_error_string_with_details(
                "TRANSACTION_NOT_FOUND",
                "Transaction not found. It may have been deleted or moved.",
                format!("Transaction id: {}", id),
            )
        })
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
        Ok(val) => {
            let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
                static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
                DEFAULT.get_or_init(AmountStyle::default)
            });
            style.format(val)
        }
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
    let mut amount = format_posting_amount(&posting.amount, &posting.commodity);
    if !posting.unit_price.trim().is_empty() && !amount.trim().is_empty() {
        amount.push_str(" @ ");
        amount.push_str(posting.unit_price.trim());
    }
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
            formatted: format_amount_styled(&amount),
            kind: "expense".to_string(),
        };
    }

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("income"))
    {
        let amount = if posting.amount.trim().is_empty() {
            balancing_amount.clone()
        } else {
            format_posting_amount(&posting.amount, &posting.commodity)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: format_display_amount(&amount, "income"),
            formatted: format_amount_styled(&amount),
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
            formatted: format_amount_styled(&amount),
            kind: "income".to_string(),
        };
    }

    if let Some(posting) = postings_with_amounts
        .first()
        .copied()
        .or_else(|| postings.first())
    {
        let amount = format_posting_amount(&posting.amount, &posting.commodity);
        let formatted = if amount.is_empty() {
            "—".to_string()
        } else {
            format_amount_styled(&amount)
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: if amount.is_empty() {
                "—".to_string()
            } else {
                amount
            },
            formatted,
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
        formatted: "—".to_string(),
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

/// Formats a numeric string using the journal's display style.
fn format_amount_styled(raw: &str) -> String {
    // Use parse_posting_amount to extract just the quantity
    let (quantity, _commodity) = parse_posting_amount(raw);
    let value = parse_amount_value(&quantity);
    let style_ref = AMOUNT_STYLE.get().unwrap_or_else(|| {
        static DEFAULT_STYLE: OnceLock<AmountStyle> = OnceLock::new();
        DEFAULT_STYLE.get_or_init(AmountStyle::default)
    });
    style_ref.format(value)
}

/// Formats the display amount sign according to the inferred transaction kind.
fn format_display_amount(amount: &str, kind: &str) -> String {
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
fn build_journal_profile(
    transactions: &[JournalTransaction],
    default_commodity: &str,
) -> JournalProfile {
    let mut cash_accounts = std::collections::HashMap::<String, usize>::new();
    let mut expense_accounts = std::collections::HashMap::<String, usize>::new();
    let mut income_accounts = std::collections::HashMap::<String, usize>::new();
    let mut transfer_accounts = std::collections::HashMap::<String, usize>::new();
    let mut investment_accounts = std::collections::HashMap::<String, usize>::new();
    let mut investment_commodities = std::collections::HashMap::<String, usize>::new();

    for transaction in transactions {
        let has_income_or_expense = transaction.postings.iter().any(|posting| {
            is_account_root(
                &posting.account,
                &["income", "revenue", "expenses", "expense"],
            )
        });

        for posting in &transaction.postings {
            let account = posting.account.trim();
            let commodity = posting.commodity.trim();
            if account.is_empty() {
                continue;
            }

            if is_account_root(account, &["expenses", "expense"]) {
                *expense_accounts.entry(account.to_string()).or_default() += 1;
            } else if is_account_root(account, &["income", "revenue"]) {
                *income_accounts.entry(account.to_string()).or_default() += 1;
            } else if is_account_root(account, &["assets", "asset", "liabilities", "liability"]) {
                if !has_income_or_expense {
                    *transfer_accounts.entry(account.to_string()).or_default() += 1;
                }

                let is_non_monetary = !commodity.is_empty() && commodity != default_commodity;
                let looks_like_investment = account.to_lowercase().contains("invest")
                    || account.to_lowercase().contains("broker")
                    || account.to_lowercase().contains("portfolio")
                    || is_non_monetary;

                if looks_like_investment && is_non_monetary {
                    *investment_accounts.entry(account.to_string()).or_default() += 1;
                    *investment_commodities
                        .entry(commodity.to_string())
                        .or_default() += 1;
                } else {
                    *cash_accounts.entry(account.to_string()).or_default() += 1;
                }
            }
        }
    }

    JournalProfile {
        default_cash_account: most_frequent(cash_accounts),
        default_expense_account: most_frequent(expense_accounts),
        default_income_account: most_frequent(income_accounts),
        default_transfer_account: most_frequent(transfer_accounts),
        default_investment_account: most_frequent(investment_accounts),
        default_investment_commodity: most_frequent(investment_commodities),
    }
}

fn is_account_root(account: &str, roots: &[&str]) -> bool {
    let normalized = account.to_lowercase();
    roots
        .iter()
        .any(|root| normalized == *root || normalized.starts_with(&format!("{}:", root)))
}

fn most_frequent(values: std::collections::HashMap<String, usize>) -> String {
    values
        .into_iter()
        .max_by(|(left_value, left_count), (right_value, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_value.cmp(left_value))
        })
        .map(|(value, _)| value)
        .unwrap_or_default()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ledgera-{}-{}", name, nanos));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn settings_for(journal_path: &Path) -> AppSettings {
        AppSettings {
            journal_path: journal_path.to_string_lossy().to_string(),
            hledger_path: "true".to_string(),
            theme: default_theme(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: String::new(),
            exclude_balances: String::new(),
            include_investments: String::new(),
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, content).expect("sample file should be written");
    }

    fn input(date: &str, description: &str) -> TransactionInput {
        TransactionInput {
            date: date.to_string(),
            status: "*".to_string(),
            code: String::new(),
            description: description.to_string(),
            postings: vec![
                PostingInput {
                    account: "expenses:test".to_string(),
                    amount: "10".to_string(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
                PostingInput {
                    account: "assets:cash".to_string(),
                    amount: String::new(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
            ],
        }
    }

    #[test]
    fn reads_transactions_from_flat_split_journal() {
        let dir = temp_dir("read-flat");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n\n2026-04-01 Main\n    assets:cash  1 EUR\n    equity:opening\n");
        write_file(
            &may,
            "2026-05-02 Split\n    expenses:office  10 EUR\n    assets:cash\n",
        );

        let summary = read_journal_summary(&main).expect("split journal should load");

        assert_eq!(summary.transactions.len(), 2);
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Split"
                && transaction.source_file.ends_with("2026-05.journal")));
    }

    #[test]
    fn reads_nested_tree_journal_with_directives_and_prices() {
        let dir = temp_dir("read-tree");
        let main = dir.join("main.journal");
        write_file(
            &main,
            "include accounts.journal\n\ninclude yearly/2026/recurring.journal\ninclude yearly/2026/2026-05.journal\n\ninclude prices/xeon.journal\n",
        );
        write_file(
            &dir.join("accounts.journal"),
            "commodity €\n  format €1.000,00\n\naccount assets:bank:fineco\n\n2026-05-16 Opening balances\n    assets:bank:fineco  €5706,51\n    equity:opening-balances\n",
        );
        write_file(
            &dir.join("yearly/2026/recurring.journal"),
            "; recurring transactions go here\n",
        );
        write_file(
            &dir.join("yearly/2026/2026-05.journal"),
            "; May 2026\n\n2026-05-20 Groceries\n    expenses:food  €25,00\n    assets:bank:fineco\n",
        );
        write_file(
            &dir.join("prices/xeon.journal"),
            "P 2026-05-16 XEON €148,70087\n",
        );

        let summary = read_journal_summary(&main).expect("tree journal should load");

        assert_eq!(summary.transactions.len(), 2);
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Opening balances"
                && transaction.source_file.ends_with("accounts.journal")));
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Groceries"
                && transaction
                    .source_file
                    .ends_with("yearly/2026/2026-05.journal")));
    }

    #[test]
    fn appends_to_existing_flat_subjournal_for_transaction_month() {
        let dir = temp_dir("append-flat-existing");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Existing\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-20", "New split transaction"))
            .expect("transaction should be routed to existing split file");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let may_content = fs::read_to_string(&may).expect("may journal should be readable");
        assert_eq!(main_content.matches("include 2026-05.journal").count(), 1);
        assert!(may_content.contains("2026-05-20 * New split transaction"));
    }

    #[test]
    fn appends_to_existing_tree_subjournal_for_transaction_month() {
        let dir = temp_dir("append-tree-existing");
        let main = dir.join("main.journal");
        let may = dir.join("yearly/2026/2026-05.journal");
        write_file(
            &main,
            "include accounts.journal\ninclude yearly/2026/2026-05.journal\ninclude prices/xeon.journal\n",
        );
        write_file(
            &dir.join("accounts.journal"),
            "account assets:bank:fineco\n",
        );
        write_file(
            &dir.join("prices/xeon.journal"),
            "P 2026-05-16 XEON €148,70087\n",
        );
        write_file(&may, "; May 2026\n");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-20", "Tree routed"))
            .expect("transaction should route to existing nested month file");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let may_content = fs::read_to_string(&may).expect("may journal should be readable");
        assert!(!main_content.contains("2026-05-20 * Tree routed"));
        assert!(may_content.contains("2026-05-20 * Tree routed"));
    }

    #[test]
    fn creates_missing_flat_tree_subjournal_and_sorted_include() {
        let dir = temp_dir("append-tree-new");
        let main = dir.join("main.journal");
        write_file(
            &main,
            "include yearly/2026/2026-04.journal\ninclude yearly/2026/2026-06.journal\n",
        );
        write_file(&dir.join("yearly/2026/2026-04.journal"), "");
        write_file(&dir.join("yearly/2026/2026-06.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-03", "Inserted tree month"))
            .expect("missing nested month journal should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let includes = main_content
            .lines()
            .filter(|line| line.starts_with("include"))
            .collect::<Vec<_>>();
        assert_eq!(
            includes,
            vec![
                "include yearly/2026/2026-04.journal",
                "include yearly/2026/2026-05.journal",
                "include yearly/2026/2026-06.journal",
            ]
        );
        assert!(fs::read_to_string(dir.join("yearly/2026/2026-05.journal"))
            .expect("new nested split file should exist")
            .contains("2026-05-03 * Inserted tree month"));
    }

    #[test]
    fn creates_missing_flat_subjournal_and_sorted_include() {
        let dir = temp_dir("append-flat-new");
        let main = dir.join("main.journal");
        write_file(&main, "include 2026-04.journal\ninclude 2026-06.journal\n");
        write_file(&dir.join("2026-04.journal"), "");
        write_file(&dir.join("2026-06.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-03", "Inserted month"))
            .expect("missing month journal should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let includes = main_content
            .lines()
            .filter(|line| line.starts_with("include"))
            .collect::<Vec<_>>();
        assert_eq!(
            includes,
            vec![
                "include 2026-04.journal",
                "include 2026-05.journal",
                "include 2026-06.journal",
            ]
        );
        assert!(fs::read_to_string(dir.join("2026-05.journal"))
            .expect("new split file should exist")
            .contains("2026-05-03 * Inserted month"));
    }

    #[test]
    fn creates_new_nested_glob_year_and_month_file() {
        let dir = temp_dir("append-nested-glob-new-year");
        let main = dir.join("main.journal");
        write_file(&main, "include yearly/2025/*.journal\n");
        write_file(&dir.join("yearly/2025/12.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-01-15", "New nested glob year"))
            .expect("new nested glob year should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        assert!(main_content.contains("include yearly/2026/*.journal"));
        assert!(fs::read_to_string(dir.join("yearly/2026/01.journal"))
            .expect("new nested glob month should exist")
            .contains("2026-01-15 * New nested glob year"));
    }

    #[test]
    fn creates_new_glob_year_and_month_file() {
        let dir = temp_dir("append-glob-new-year");
        let main = dir.join("main.journal");
        write_file(&main, "include 2025/*.journal\n");
        write_file(&dir.join("2025/12.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-01-15", "New glob year"))
            .expect("new glob year should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        assert!(main_content.contains("include 2026/*.journal"));
        assert!(fs::read_to_string(dir.join("2026/01.journal"))
            .expect("new glob month should exist")
            .contains("2026-01-15 * New glob year"));
    }

    #[test]
    fn update_and_delete_target_source_subjournal() {
        let dir = temp_dir("mutate-source");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Original\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);
        let summary = read_journal_summary(&main).expect("summary should load");
        let transaction_id = summary.transactions[0].id.clone();

        update_transaction_for_settings(
            &settings,
            &transaction_id,
            &input("2026-05-02", "Updated"),
        )
        .expect("source subjournal transaction should update");
        let updated_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(updated_content.contains("2026-05-02 * Updated"));
        assert!(!updated_content.contains("Original"));

        let updated_summary = read_journal_summary(&main).expect("updated summary should load");
        delete_transaction_for_settings(&settings, &updated_summary.transactions[0].id)
            .expect("source subjournal transaction should delete");
        let deleted_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(!deleted_content.contains("2026-05-02"));
    }

    #[test]
    fn builds_frequency_based_journal_profile() {
        let transactions = vec![
            JournalTransaction {
                id: "test:1".to_string(),
                source_file: "test".to_string(),
                date: "2026-05-16".to_string(),
                status: String::new(),
                code: String::new(),
                description: "Groceries".to_string(),
                postings: vec![
                    JournalPosting {
                        account: "expenses:food".to_string(),
                        amount: "25".to_string(),
                        commodity: "€".to_string(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                    JournalPosting {
                        account: "assets:bank:fineco".to_string(),
                        amount: String::new(),
                        commodity: String::new(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                ],
                display: TransactionDisplay {
                    account: "expenses:food".to_string(),
                    amount: "-€25".to_string(),
                    formatted: "-25,00".to_string(),
                    kind: "expense".to_string(),
                },
                raw: String::new(),
                start_line: 1,
                end_line: 3,
            },
            JournalTransaction {
                id: "test:2".to_string(),
                source_file: "test".to_string(),
                date: "2026-05-17".to_string(),
                status: String::new(),
                code: String::new(),
                description: "Buy fund".to_string(),
                postings: vec![
                    JournalPosting {
                        account: "assets:investments:xeon".to_string(),
                        amount: "10".to_string(),
                        commodity: "XEON".to_string(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                    JournalPosting {
                        account: "assets:bank:fineco".to_string(),
                        amount: "1487".to_string(),
                        commodity: "€".to_string(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                ],
                display: TransactionDisplay {
                    account: "assets:investments:xeon".to_string(),
                    amount: "10 XEON".to_string(),
                    formatted: "10 XEON".to_string(),
                    kind: "transfer".to_string(),
                },
                raw: String::new(),
                start_line: 5,
                end_line: 7,
            },
        ];

        let profile = build_journal_profile(&transactions, "€");

        assert_eq!(profile.default_cash_account, "assets:bank:fineco");
        assert_eq!(profile.default_expense_account, "expenses:food");
        assert_eq!(
            profile.default_investment_account,
            "assets:investments:xeon"
        );
        assert_eq!(profile.default_investment_commodity, "XEON");
    }

    #[test]
    fn configured_hledger_path_overrides_detection() {
        let settings = AppSettings {
            journal_path: String::new(),
            hledger_path: "/custom/bin/hledger".to_string(),
            theme: default_theme(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: String::new(),
            exclude_balances: String::new(),
            include_investments: String::new(),
        };

        assert_eq!(hledger_executable(&settings), "/custom/bin/hledger");
    }

    #[test]
    fn formats_amount_with_italian_locale() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ",",
                "asdigitgroups": [".", [3]],
                "asprecision": 2
            }
        });
        assert_eq!(format_hledger_amount(33452.31, &bal), "33.452,31");
        assert_eq!(format_hledger_amount(-1858.0, &bal), "-1.858,00");
        assert_eq!(format_hledger_amount(2303.51, &bal), "2.303,51");
    }

    #[test]
    fn formats_amount_with_english_locale() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": [",", [3]],
                "asprecision": 2
            }
        });
        assert_eq!(format_hledger_amount(1234567.89, &bal), "1,234,567.89");
    }

    #[test]
    fn formats_amount_without_digit_groups() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": null,
                "asprecision": 0
            }
        });
        assert_eq!(format_hledger_amount(209.0, &bal), "209");
    }

    #[test]
    fn formats_posting_with_unit_price() {
        let posting = PostingInput {
            account: "assets:investments:etf".to_string(),
            amount: "10".to_string(),
            commodity: "VWCE".to_string(),
            unit_price: "150 EUR".to_string(),
            comment: String::new(),
        };
        let result = format_posting(&posting);
        assert!(result.contains("@ 150 EUR"));
        assert!(result.contains("10.00 VWCE"));
        assert!(result.contains("assets:investments:etf"));
    }

    #[test]
    fn formats_posting_without_unit_price_when_empty() {
        let posting = PostingInput {
            account: "expenses:food".to_string(),
            amount: "25".to_string(),
            commodity: "EUR".to_string(),
            unit_price: String::new(),
            comment: String::new(),
        };
        let result = format_posting(&posting);
        assert!(!result.contains('@'));
        assert!(result.contains("25.00 EUR"));
    }

    #[test]
    fn formats_posting_with_unit_price_and_comment() {
        let posting = PostingInput {
            account: "assets:investments:etf".to_string(),
            amount: "5".to_string(),
            commodity: "BTC".to_string(),
            unit_price: "45000 USD".to_string(),
            comment: "limit order".to_string(),
        };
        let result = format_posting(&posting);
        assert!(result.contains("@ 45000 USD"));
        assert!(result.contains("; limit order"));
    }
}
