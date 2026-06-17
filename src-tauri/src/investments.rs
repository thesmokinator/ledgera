use crate::{
    amount_style::{resolve_currency_display, style_for_commodity},
    app_error::to_error_string_with_details,
    balances::{parse_balance_output, Balance},
    hledger::hledger_executable,
    journal::files::require_journal_path,
    logs,
    settings::{read_settings, CommoditySymbolMapping},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    process::Command,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
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
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
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
        tauri::async_runtime::spawn_blocking(move || {
            cmd.output().map_err(|e| {
                to_error_string_with_details(
                    "hledger_balance_failed",
                    "Unable to run hledger balance for investments.",
                    e.to_string(),
                )
            })
        })
            .await
            .map_err(|error| {
                to_error_string_with_details(
                    "hledger_balance_failed",
                    "Unable to run hledger balance for investments.",
                    error.to_string(),
                )
            })??;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_balance_output(&stdout, &settings, false)
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
    error: Option<String>,
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
            .map(|h| {
                let commodity_style = style_for_commodity(&h.commodity);
                InvestmentOverview {
                    commodity: h.commodity.clone(),
                    account: h.account,
                    quantity: h.amount,
                    quantity_formatted: commodity_style.format(h.amount),
                    price: None,
                    price_formatted: None,
                    currency: None,
                    market_value_formatted: None,
                    tint: h.tint,
                    error: None,
                }
            })
            .collect());
    }

    let mappings: Vec<CommoditySymbolMapping> = settings.commodity_symbols.clone();
    let symbols: Vec<String> = holdings.iter().map(|h| h.commodity.clone()).collect();
    let prices = fetch_prices(app.clone(), &symbols, &mappings).await;

    Ok(holdings
        .into_iter()
        .map(|h| {
            let commodity_style = style_for_commodity(&h.commodity);
            let price_info = prices.get(&h.commodity);
            let (price, price_formatted, currency, market_value_formatted, error) = match price_info
            {
                Some(Ok(info)) => {
                    let mv = h.amount * info.price;
                    let resolved_currency = resolve_currency_display(&info.currency);
                    (
                        Some(info.price),
                        Some(info.formatted.clone()),
                        Some(resolved_currency.clone()),
                        Some(commodity_style.format_amount(mv, &resolved_currency)),
                        None,
                    )
                }
                Some(Err(err)) => (None, None, None, None, Some(err.clone())),
                None => (None, None, None, None, None),
            };
            InvestmentOverview {
                commodity: h.commodity.clone(),
                account: h.account,
                quantity: h.amount,
                quantity_formatted: commodity_style.format(h.amount),
                price,
                price_formatted,
                currency,
                market_value_formatted,
                tint: h.tint,
                error,
            }
        })
        .collect())
}

/// Session-level price cache with timestamp.
static PRICE_CACHE: OnceLock<Mutex<HashMap<String, (Instant, PriceInfo)>>> = OnceLock::new();
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

fn price_cache() -> &'static Mutex<HashMap<String, (Instant, PriceInfo)>> {
    PRICE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fetches current prices from Yahoo Finance for a list of symbols,
/// using session-level caching and respecting rate limits.
async fn fetch_prices(
    app: AppHandle,
    symbols: &[String],
    mappings: &[CommoditySymbolMapping],
) -> HashMap<String, Result<PriceInfo, String>> {
    let mut prices = HashMap::new();

    // Build mapping lookup: commodity → yahoo symbol
    let mapping: HashMap<&str, &str> = mappings
        .iter()
        .map(|m| (m.commodity.as_str(), m.yahoo_symbol.as_str()))
        .collect();

    for symbol in symbols {
        // Check session cache first
        {
            let cache = price_cache().lock().unwrap();
            if let Some((cached_at, info)) = cache.get(symbol) {
                if cached_at.elapsed() < CACHE_TTL {
                    prices.insert(symbol.clone(), Ok(info.clone()));
                    continue;
                }
            }
        }

        let yahoo_symbol = mapping
            .get(symbol.as_str())
            .copied()
            .unwrap_or(symbol.as_str());

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}",
            yahoo_symbol
        );

        let result = fetch_single_price(&url).await;

        let price_result = match result {
            Ok((price, currency)) => {
                let style = style_for_commodity(symbol);
                let resolved_currency = resolve_currency_display(&currency);
                let formatted = style.format_amount(price, &resolved_currency);
                let info = PriceInfo {
                    price,
                    currency,
                    formatted,
                };
                let mut cache = price_cache().lock().unwrap();
                cache.insert(symbol.clone(), (Instant::now(), info.clone()));
                Ok(info)
            }
            Err(err) => {
                logs::log_warn(
                    &app,
                    "price_fetch_failed",
                    &format!(
                        "Failed to fetch price for {} (Yahoo: {}).",
                        symbol, yahoo_symbol
                    ),
                    &err,
                );
                Err(err)
            }
        };

        prices.insert(symbol.clone(), price_result);
    }

    prices
}

/// Fetches a single price from Yahoo Finance v8 API.
async fn fetch_single_price(url: &str) -> Result<(f64, String), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let body_text = response.text().await.unwrap_or_default();
    let json: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Invalid JSON: {e}"))?;

    let meta = &json["chart"]["result"][0]["meta"];
    let price = meta["regularMarketPrice"]
        .as_f64()
        .ok_or_else(|| "Missing regularMarketPrice".to_string())?;
    let currency = meta["currency"].as_str().unwrap_or("USD").to_string();

    Ok((price, currency))
}
