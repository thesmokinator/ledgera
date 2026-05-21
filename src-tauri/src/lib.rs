//! Application backend for Ledgera.
//!
//! The Rust layer owns journal access, conservative transaction edits, settings
//! persistence, and integration with the official hledger CLI.

mod accounts;
mod amount_format;
mod app_error;
mod hledger;
mod journal;
mod logs;
mod settings;
mod updates;

use accounts::{build_accounts_overview, AccountsOverview};
use amount_format::{AmountFormatConfig, CommodityPosition};
use app_error::{to_error_string, to_error_string_with_details};

use hledger::hledger_executable;
#[cfg(test)]
use journal::autocomplete::build_journal_profile;
#[cfg(test)]
use journal::parser::{format_posting, summarize_transaction};
#[cfg(test)]
use journal::types::{
    JournalPosting, JournalTransaction, PostingInput, TransactionDisplay, TransactionFlow,
};
use journal::{
    autocomplete::collect_declared_commodities,
    files::{load_journal_files, require_journal_path, JournalFile},
    parser::load_transactions_from_journal_via_files,
    summary::build_dashboard_summary,
    transactions::{
        create_transaction_for_settings, delete_transaction_for_settings,
        update_transaction_for_settings,
    },
    types::{JournalSummary, TransactionInput},
};
use serde::{Deserialize, Serialize};
use settings::{read_settings, AppSettings};
#[cfg(test)]
use std::{env, fs, path::PathBuf};
use std::{path::Path, process::Command, sync::OnceLock};
use tauri::AppHandle;

static AMOUNT_STYLE: OnceLock<AmountStyle> = OnceLock::new();

// ── Holdings & Prices ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PriceInfo {
    price: f64,
    currency: String,
    formatted: String,
}

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
                    "hledger_balance_failed",
                    "Unable to run hledger balance for investments.",
                    error.to_string(),
                )
            })??;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_balance_output(&app, &stdout, &settings, false)
}

/// Aggregated investment row with price and market value pre-computed
/// so the frontend never has to combine data client-side.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvestmentOverview {
    commodity: String,
    account: String,
    quantity: f64,
    quantity_formatted: String,
    price: Option<f64>,
    price_formatted: Option<String>,
    currency: Option<String>,
    market_value_formatted: Option<String>,
    tint: String,
}

#[tauri::command]
async fn get_investments_overview(app: AppHandle) -> Result<Vec<InvestmentOverview>, String> {
    let settings = read_settings(&app)?;
    let holdings = get_investments(app.clone()).await?;
    if holdings.is_empty() || !settings.fetch_prices {
        return Ok(holdings
            .into_iter()
            .map(|h| InvestmentOverview {
                commodity: h.commodity.clone(),
                account: h.account,
                quantity: h.amount,
                quantity_formatted: h.formatted,
                price: None,
                price_formatted: None,
                currency: None,
                market_value_formatted: None,
                tint: h.tint,
            })
            .collect());
    }

    let symbols: Vec<String> = holdings.iter().map(|h| h.commodity.clone()).collect();
    let prices = fetch_prices(app.clone(), symbols).await.unwrap_or_default();

    let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
        static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
        DEFAULT.get_or_init(AmountStyle::default)
    });

    Ok(holdings
        .into_iter()
        .map(|h| {
            let price_info = prices.get(&h.commodity);
            let (price, price_formatted, currency, market_value_formatted) =
                if let Some(info) = price_info {
                    let mv = h.amount * info.price;
                    (
                        Some(info.price),
                        Some(info.formatted.clone()),
                        Some(info.currency.clone()),
                        Some(format!("{} {}", info.currency, style.format(mv))),
                    )
                } else {
                    (None, None, None, None)
                };
            InvestmentOverview {
                commodity: h.commodity.clone(),
                account: h.account,
                quantity: h.amount,
                quantity_formatted: h.formatted,
                price,
                price_formatted,
                currency,
                market_value_formatted,
                tint: h.tint,
            }
        })
        .collect())
}

fn parse_balance_output(
    _app: &AppHandle,
    stdout: &str,
    settings: &AppSettings,
    apply_exclude: bool,
) -> Result<Vec<Balance>, String> {
    let raw: Vec<serde_json::Value> = serde_json::from_str(stdout).map_err(|error| {
        to_error_string_with_details(
            "hledger_balance_parse_failed",
            "Unable to parse hledger balance output.",
            error.to_string(),
        )
    })?;

    let rows = raw.first().and_then(|v| v.as_array()).ok_or_else(|| {
        to_error_string(
            "hledger_balance_parse_failed",
            "Unexpected hledger balance JSON structure.",
        )
    })?;

    let exclude_set: std::collections::HashSet<&str> = settings
        .exclude_balances
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

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
                // If hledger didn't report a commodity, fall back to the
                // user-configured default so the formatted string always
                // includes a commodity when one is expected.
                let effective_commodity = if comm.is_empty() {
                    settings.default_commodity.trim().to_string()
                } else {
                    comm
                };
                let fmt = format_hledger_display_amount(qty, &effective_commodity, bal);
                (qty, effective_commodity, fmt)
            } else {
                let default_comm = settings.default_commodity.trim().to_string();
                let fmt = if default_comm.is_empty() {
                    "0".to_string()
                } else {
                    let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
                        static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
                        DEFAULT.get_or_init(AmountStyle::default)
                    });
                    style.format_amount(0.0, &default_comm)
                };
                (0.0, default_comm, fmt)
            };
        let tint = if amount < 0.0 {
            "negative".to_string()
        } else if amount > 0.0 {
            "positive".to_string()
        } else {
            "neutral".to_string()
        };
        result.push(Balance {
            account,
            amount,
            commodity,
            formatted,
            tint,
        });
    }
    Ok(result)
}

/// Fetches current prices from Yahoo Finance for a list of symbols.
async fn fetch_prices(
    app: AppHandle,
    symbols: Vec<String>,
) -> Result<std::collections::HashMap<String, PriceInfo>, String> {
    let settings = read_settings(&app)?;

    if !settings.fetch_prices {
        return Err(to_error_string(
            "prices_disabled",
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
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(error) => {
                logs::log_event_with_details(
                    &app,
                    "error",
                    "price_fetch_failed",
                    &format!("HTTP request failed for {}", symbol),
                    Some(error.to_string()),
                );
            }
        }
    }
    Ok(prices)
}

/// Display style for amounts (decimal mark, digit grouping, precision, commodity placement).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AmountStyle {
    decimal_mark: String,
    digit_separator: String,
    digit_groups: Vec<usize>,
    precision: usize,
    commodity_position: String,
    commodity_spaced: bool,
}

impl Default for AmountStyle {
    fn default() -> Self {
        Self {
            decimal_mark: ".".to_string(),
            digit_separator: ",".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: "left".to_string(),
            commodity_spaced: false,
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
        let commodity_position =
            parse_hledger_commodity_position(style).unwrap_or(CommodityPosition::Left);
        let commodity_spaced = parse_hledger_commodity_spaced(style).unwrap_or(false);
        Self {
            decimal_mark,
            digit_separator,
            digit_groups,
            precision,
            commodity_position: match commodity_position {
                CommodityPosition::Left => "left".to_string(),
                CommodityPosition::Right => "right".to_string(),
            },
            commodity_spaced,
        }
    }

    fn to_format_config(&self) -> AmountFormatConfig {
        AmountFormatConfig {
            decimal_mark: self.decimal_mark.clone(),
            digit_separator: self.digit_separator.clone(),
            digit_groups: self.digit_groups.clone(),
            precision: self.precision,
            commodity_position: if self.commodity_position == "right" {
                CommodityPosition::Right
            } else {
                CommodityPosition::Left
            },
            commodity_spaced: self.commodity_spaced,
        }
    }

    fn format(&self, amount: f64) -> String {
        self.to_format_config().format_quantity(amount)
    }

    fn format_amount(&self, amount: f64, commodity: &str) -> String {
        self.to_format_config().format_amount(amount, commodity)
    }
}

fn parse_hledger_commodity_position(style: &serde_json::Value) -> Option<CommodityPosition> {
    let value = style
        .get("ascommodityside")
        .or_else(|| style.get("ascommoditySide"))
        .or_else(|| style.get("commoditySide"))?;
    let side = value.as_str().unwrap_or_default().to_lowercase();
    if side.starts_with('r') {
        Some(CommodityPosition::Right)
    } else if side.starts_with('l') {
        Some(CommodityPosition::Left)
    } else {
        None
    }
}

fn parse_hledger_commodity_spaced(style: &serde_json::Value) -> Option<bool> {
    style
        .get("ascommodityspaced")
        .or_else(|| style.get("ascommoditySpaced"))
        .or_else(|| style.get("commoditySpaced"))
        .and_then(|value| value.as_bool())
}

/// Returns account balances from hledger as a hierarchical tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Balance {
    pub(crate) account: String,
    pub(crate) amount: f64,
    pub(crate) commodity: String,
    pub(crate) formatted: String,
    pub(crate) tint: String,
}

/// Formats a number according to hledger display style (decimal mark, digit groups).
#[cfg(test)]
fn format_hledger_amount(amount: f64, bal: &serde_json::Value) -> String {
    AmountStyle::from_hledger_json(bal).format(amount)
}

/// Formats a number and commodity according to hledger display style.
fn format_hledger_display_amount(amount: f64, commodity: &str, bal: &serde_json::Value) -> String {
    AmountStyle::from_hledger_json(bal).format_amount(amount, commodity)
}

fn load_balances_for_settings(
    app: &AppHandle,
    settings: &AppSettings,
    apply_exclude: bool,
) -> Result<Vec<Balance>, String> {
    let journal_path = require_journal_path(settings)?;
    let executable = hledger_executable(settings);

    let output = Command::new(&executable)
        .arg("-f")
        .arg(&journal_path)
        .arg("balance")
        .arg("-O")
        .arg("json")
        .output()
        .map_err(|error| {
            to_error_string_with_details(
                "hledger_balance_failed",
                "Unable to run hledger balance.",
                error.to_string(),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        logs::log_event_with_details(
            app,
            "error",
            "balance_failed",
            "hledger balance failed",
            Some(stderr.clone()),
        );
        return Err(to_error_string_with_details(
            "hledger_balance_failed",
            "hledger balance command failed.",
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_balance_output(app, &stdout, settings, apply_exclude)
}

#[tauri::command]
async fn get_balances(app: AppHandle) -> Result<Vec<Balance>, String> {
    let settings = read_settings(&app)?;
    let app_for_task = app.clone();
    let settings_for_task = settings.clone();

    tauri::async_runtime::spawn_blocking(move || {
        load_balances_for_settings(&app_for_task, &settings_for_task, true)
    })
    .await
    .map_err(|error| {
        to_error_string_with_details(
            "hledger_balance_failed",
            "Unable to run hledger balance.",
            error.to_string(),
        )
    })?
}

#[tauri::command]
async fn get_accounts_overview(
    app: AppHandle,
    activity_range: String,
) -> Result<AccountsOverview, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let summary = read_journal_summary(&journal_path)?;
    let app_for_task = app.clone();
    let settings_for_task = settings.clone();

    let balances = tauri::async_runtime::spawn_blocking(move || {
        load_balances_for_settings(&app_for_task, &settings_for_task, true)
    })
    .await
    .map_err(|error| {
        to_error_string_with_details(
            "hledger_balance_failed",
            "Unable to run hledger balance for accounts overview.",
            error.to_string(),
        )
    })??;

    Ok(build_accounts_overview(
        &summary.transactions,
        balances,
        &activity_range,
    ))
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
fn update_transaction(
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
fn delete_transaction(app: AppHandle, id: String) -> Result<JournalSummary, String> {
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

/// Starts the Tauri application and registers backend commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            logs::cleanup_old_logs(&app.handle());

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
            settings::get_app_settings,
            settings::update_app_settings,
            hledger::check_hledger,
            updates::check_for_updates,
            journal::autocomplete::get_autocomplete_suggestions,
            list_transactions,
            journal::search::search_journal,
            create_transaction,
            update_transaction,
            delete_transaction,
            logs::get_logs,
            logs::clear_logs,
            get_investments_overview,
            get_balances,
            get_accounts_overview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
    let trimmed = fmt.trim();
    let first_digit = trimmed.find(|c: char| c.is_ascii_digit())?;
    let last_digit = trimmed.rfind(|c: char| c.is_ascii_digit())?;
    let prefix = &trimmed[..first_digit];
    let suffix = &trimmed[last_digit + 1..];
    let num_part = &trimmed[first_digit..=last_digit];
    let commodity_position = if !suffix.trim().is_empty() {
        "right"
    } else {
        "left"
    };
    let commodity_spaced = if commodity_position == "right" {
        suffix.chars().next().is_some_and(char::is_whitespace)
    } else {
        prefix.chars().last().is_some_and(char::is_whitespace)
    };

    let chars: Vec<char> = num_part.chars().collect();
    let len = chars.len();

    let mut trailing_digits = 0;
    for i in (0..len).rev() {
        if chars[i].is_ascii_digit() {
            trailing_digits += 1;
        } else {
            break;
        }
    }

    if trailing_digits == 0 || trailing_digits == len {
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
            commodity_position: commodity_position.to_string(),
            commodity_spaced,
        });
    }

    let decimal_mark = chars[len - trailing_digits - 1].to_string();
    let int_part: String = chars[..len - trailing_digits - 1].iter().collect();
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
        commodity_position: commodity_position.to_string(),
        commodity_spaced,
    })
}

pub(crate) fn read_journal_summary(journal_path: &Path) -> Result<JournalSummary, String> {
    let files = load_journal_files(journal_path)?;
    let file_count = files.len();
    let total_size_bytes: u64 = files.iter().map(|f| f.content.len() as u64).sum();

    let amount_style = parse_amount_style(&files, "€");
    let _ = AMOUNT_STYLE.set(amount_style.clone());

    let transactions = load_transactions_from_journal_via_files(&files)?;
    let commodities = collect_declared_commodities(&files);
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

    fn settings_for(path: &Path) -> AppSettings {
        AppSettings {
            journal_path: path.to_string_lossy().to_string(),
            hledger_path: "true".to_string(),
            theme: "system".to_string(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: String::new(),
            exclude_balances: String::new(),
            include_investments: String::new(),
            prefill_postings: false,
        }
    }

    fn posting(account: &str, amount: &str, commodity: &str) -> JournalPosting {
        JournalPosting {
            account: account.to_string(),
            amount: amount.to_string(),
            commodity: commodity.to_string(),
            comment: String::new(),
            raw: String::new(),
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, content).expect("file should be written");
    }

    #[test]
    fn collects_only_declared_commodities() {
        let files = vec![JournalFile {
            path: PathBuf::from("main.journal"),
            content: "commodity €\n  format €1.000,00\ncommodity XEON\n\n2026-05-16 Opening balances\n    assets:investments:xeon  209 XEON @ €148,70087\n    equity:opening-balances\n"
                .to_string(),
        }];

        assert_eq!(collect_declared_commodities(&files), vec!["XEON", "€"]);
    }

    #[test]
    fn summarizes_income_flow_from_income_to_asset_with_implicit_posting() {
        let display = summarize_transaction(&[
            posting("assets:bank:postepay", "5.20", "€"),
            posting("income:other", "", ""),
        ]);

        assert_eq!(display.kind, "income");
        assert_eq!(display.flow.from, vec!["income:other"]);
        assert_eq!(display.flow.to, vec!["assets:bank:postepay"]);
    }

    #[test]
    fn summarizes_expense_flow_from_asset_to_expense_with_implicit_posting() {
        let display = summarize_transaction(&[
            posting("expenses:food", "20", "€"),
            posting("assets:bank", "", ""),
        ]);

        assert_eq!(display.kind, "expense");
        assert_eq!(display.flow.from, vec!["assets:bank"]);
        assert_eq!(display.flow.to, vec!["expenses:food"]);
    }

    #[test]
    fn summarizes_transfer_flow_from_negative_to_positive_posting() {
        let display = summarize_transaction(&[
            posting("assets:wallet", "-50", "€"),
            posting("assets:bank", "50", "€"),
        ]);

        assert_eq!(display.kind, "transfer");
        assert_eq!(display.flow.from, vec!["assets:wallet"]);
        assert_eq!(display.flow.to, vec!["assets:bank"]);
    }

    #[test]
    fn summarizes_split_expense_flow_and_total_amount() {
        let display = summarize_transaction(&[
            posting("expenses:shopping", "26.58", "€"),
            posting("expenses:shopping:gifts", "19.99", "€"),
            posting("assets:bank:fineco", "", ""),
        ]);

        assert_eq!(display.kind, "expense");
        assert_eq!(display.amount, "-€46.57");
        assert_eq!(display.formatted, "€46.57");
        assert_eq!(display.flow.from, vec!["assets:bank:fineco"]);
        assert_eq!(
            display.flow.to,
            vec!["expenses:shopping", "expenses:shopping:gifts"]
        );
    }

    #[test]
    fn summarizes_opening_balance_amount_by_positive_commodities() {
        let display = summarize_transaction(&[
            posting("assets:bank:fineco", "5.706,51", "€"),
            posting("assets:bank:postepay", "890,05", "€"),
            posting("assets:investments:xeon", "209", "XEON @ €148,70087"),
            posting("equity:opening-balances", "", ""),
        ]);

        assert_eq!(display.kind, "transfer");
        assert_eq!(display.amount, "€6596.56 + 209 XEON");
        assert_eq!(display.formatted, "€6,596.56 + 209 XEON");
        assert_eq!(display.flow.from, vec!["equity:opening-balances"]);
        assert_eq!(
            display.flow.to,
            vec![
                "assets:bank:fineco",
                "assets:bank:postepay",
                "assets:investments:xeon"
            ]
        );
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
                    tint: "negative".to_string(),
                    flow: TransactionFlow {
                        from: vec!["assets:bank:fineco".to_string()],
                        to: vec!["expenses:food".to_string()],
                    },
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
                    tint: "positive".to_string(),
                    flow: TransactionFlow {
                        from: Vec::new(),
                        to: vec![
                            "assets:investments:xeon".to_string(),
                            "assets:bank:fineco".to_string(),
                        ],
                    },
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
    fn parses_right_side_commodity_from_format_directive() {
        let style = parse_format_directive("1.000,00 €").expect("format directive should parse");

        assert_eq!(style.commodity_position, "right");
        assert!(style.commodity_spaced);
        assert_eq!(style.format_amount(5317.55, "€"), "5.317,55 €");
    }

    #[test]
    fn parses_left_side_commodity_from_format_directive() {
        let style = parse_format_directive("$1,000.00").expect("format directive should parse");

        assert_eq!(style.commodity_position, "left");
        assert!(!style.commodity_spaced);
        assert_eq!(style.format_amount(5317.55, "$"), "$5,317.55");
    }

    #[test]
    fn formats_display_amount_with_right_side_commodity_style() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ",",
                "asdigitgroups": [".", [3]],
                "asprecision": 2,
                "ascommodityside": "R",
                "ascommodityspaced": true
            }
        });
        assert_eq!(
            format_hledger_display_amount(5317.55, "€", &bal),
            "5.317,55 €"
        );
        assert_eq!(
            format_hledger_display_amount(-37744.98, "€", &bal),
            "-37.744,98 €"
        );
    }

    #[test]
    fn formats_display_amount_with_left_side_commodity_style() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": [",", [3]],
                "asprecision": 2,
                "ascommodityside": "L",
                "ascommodityspaced": false
            }
        });
        assert_eq!(
            format_hledger_display_amount(1234.56, "$", &bal),
            "$1,234.56"
        );
        assert_eq!(
            format_hledger_display_amount(-1234.56, "$", &bal),
            "-$1,234.56"
        );
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
