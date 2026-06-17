use crate::{
    amount_style::{format_hledger_display_amount, AmountStyle},
    app_error::{to_error_string, to_error_string_with_details},
    hledger::hledger_executable,
    journal::files::require_journal_path,
    logs,
    settings::{read_settings, AppSettings},
    AMOUNT_STYLE,
};
use serde::Serialize;
use std::process::Command;
use tauri::AppHandle;

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

pub(crate) fn parse_balance_output(
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
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    // Aggregate by (account, commodity) — hledger may return multiple lots
    // for the same account when cost annotations (@) are present.
    use std::collections::HashMap;
    struct Accum {
        qty: f64,
        bal: serde_json::Value,
    }
    let mut agg: HashMap<(String, String), Accum> = HashMap::new();

    for row in rows {
        let arr = match row.as_array() {
            Some(a) if a.len() >= 4 => a,
            _ => continue,
        };
        let account = arr[0].as_str().unwrap_or("").to_string();
        if apply_exclude && exclude_set.contains(account.as_str()) {
            continue;
        }
        if let Some(amounts) = arr[3].as_array() {
            for bal in amounts {
                let comm = bal["acommodity"].as_str().unwrap_or("").to_string();
                if AMOUNT_STYLE.get().is_none() {
                    let style = AmountStyle::from_hledger_json(bal);
                    if AMOUNT_STYLE.set(style).is_err() {
                        eprintln!("[warn] AMOUNT_STYLE already set by another balance query");
                    }
                }
                let qty = bal["aquantity"]
                    .get("floatingPoint")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let effective_commodity = if comm.is_empty() {
                    settings.default_commodity.trim().to_string()
                } else {
                    comm
                };
                let key = (account.clone(), effective_commodity);
                match agg.get_mut(&key) {
                    Some(acc) => {
                        acc.qty += qty;
                    }
                    None => {
                        agg.insert(key, Accum {
                            qty,
                            bal: bal.clone(),
                        });
                    }
                }
            }
        }
    }

    let mut result: Vec<Balance> = agg
        .into_iter()
        .map(|((account, commodity), accum)| {
            let effective_comm = if commodity.is_empty() {
                settings.default_commodity.trim().to_string()
            } else {
                commodity
            };
            let formatted = if effective_comm.is_empty() {
                accum.qty.to_string()
            } else {
                format_hledger_display_amount(accum.qty, &effective_comm, &accum.bal)
            };
            let tint = crate::tint(accum.qty).to_string();
            Balance {
                account,
                amount: accum.qty,
                commodity: effective_comm,
                formatted,
                tint,
            }
        })
        .collect();

    result.sort_by(|a, b| a.account.cmp(&b.account));
    Ok(result)
}
pub(crate) fn load_balances_for_settings(
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
        logs::log_error(
            app,
            "balance_failed",
            "hledger balance command failed.",
            stderr.clone(),
        );
        return Err(to_error_string_with_details(
            "hledger_balance_failed",
            "hledger balance command failed.",
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_balance_output(&stdout, settings, apply_exclude)
}

#[tauri::command]
pub(crate) async fn get_balances(app: AppHandle) -> Result<Vec<Balance>, String> {
    let settings = read_settings(&app)?;

    crate::run_blocking(
        "hledger_balance_failed",
        "Unable to run hledger balance.",
        move || load_balances_for_settings(&app, &settings, true),
    )
    .await
}
