use crate::{
    amount_style::AmountStyle,
    app_error::{to_error_string, to_error_string_with_details},
    balances::{parse_balance_output, Balance},
    hledger::hledger_executable,
    journal::files::require_journal_path,
    logs,
    settings::read_settings,
    AMOUNT_STYLE,
};
use serde::Serialize;
use std::{process::Command, sync::OnceLock};
use tauri::AppHandle;

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
pub(crate) struct InvestmentOverview {
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
pub(crate) async fn get_investments_overview(
    app: AppHandle,
) -> Result<Vec<InvestmentOverview>, String> {
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
