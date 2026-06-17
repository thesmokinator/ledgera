use crate::{
    amount_style::{format_hledger_display_amount, AmountStyle},
    app_error::to_error_string_with_details,
    hledger::hledger_executable,
    journal::files::require_journal_path,
    logs,
    settings::read_settings,
    AMOUNT_STYLE,
};
use chrono::Local;
use serde::Serialize;
use std::process::Command;
use tauri::AppHandle;

use crate::reports::{self, run_command_with_timeout, ReportDateRange, REPORT_TIMEOUT};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BudgetPeriodAmount {
    pub period: String,
    pub actual: f64,
    pub budget: f64,
    pub remaining: f64,
    pub pct_used: f64,
    pub commodity: String,
    pub actual_formatted: String,
    pub budget_formatted: String,
    pub remaining_formatted: String,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BudgetRow {
    pub account: String,
    pub periods: Vec<BudgetPeriodAmount>,
    pub total_actual: f64,
    pub total_budget: f64,
    pub total_remaining: f64,
    pub total_pct_used: f64,
    pub commodity: String,
    pub total_actual_formatted: String,
    pub total_budget_formatted: String,
    pub total_remaining_formatted: String,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BudgetReport {
    pub period_columns: Vec<String>,
    pub rows: Vec<BudgetRow>,
}

fn amount_quantity(raw: &serde_json::Value) -> f64 {
    raw["aquantity"]
        .get("floatingPoint")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

fn amount_commodity(raw: &serde_json::Value) -> String {
    raw["acommodity"].as_str().unwrap_or("").to_string()
}

fn format_budget_amount(qty: f64, commodity: &str, raw_bal: &serde_json::Value) -> String {
    if commodity.is_empty() {
        return format!("{:.2}", qty);
    }
    format_hledger_display_amount(qty, commodity, raw_bal)
}

fn period_label(period: &serde_json::Value) -> String {
    let dates = period.as_array();
    let start = dates
        .and_then(|d| d.first())
        .and_then(|date| date["contents"].as_str())
        .unwrap_or_default();
    if start.len() >= 10 && start.ends_with("-01") {
        start[..7].to_string()
    } else if start.len() >= 10 {
        start[..10].to_string()
    } else {
        start.to_string()
    }
}

fn extract_period_amounts(
    period_data: &[serde_json::Value],
    _commodity_filter: Option<&str>,
) -> Vec<(f64, String, serde_json::Value)> {
    let mut result = Vec::new();
    for amt in period_data {
        let comm = amount_commodity(amt);
        let qty = amount_quantity(amt);
        result.push((qty, comm, amt.clone()));
    }
    result
}

fn aggregate_period(
    period_label: &str,
    actual_amounts: &[(f64, String, serde_json::Value)],
    budget_amounts: &[(f64, String, serde_json::Value)],
) -> BudgetPeriodAmount {
    let actual: f64 = actual_amounts.iter().map(|(q, _, _)| q).sum();
    let budget: f64 = budget_amounts.iter().map(|(q, _, _)| q).sum();
    let remaining = budget - actual;
    let pct_used = if budget != 0.0 {
        ((actual / budget) * 100.0).clamp(-9999.0, 9999.0)
    } else {
        f64::NAN
    };

    let commodity = actual_amounts
        .first()
        .or(budget_amounts.first())
        .map(|(_, c, _)| c.clone())
        .unwrap_or_default();

    let raw_bal = actual_amounts
        .first()
        .or(budget_amounts.first())
        .map(|(_, _, r)| r.clone())
        .unwrap_or(serde_json::Value::Null);

    let tint = if pct_used.is_nan() {
        "neutral".to_string()
    } else if pct_used > 100.0 {
        "negative".to_string()
    } else if pct_used > 80.0 {
        "neutral".to_string()
    } else {
        "positive".to_string()
    };

    BudgetPeriodAmount {
        period: period_label.to_string(),
        actual,
        budget,
        remaining,
        pct_used,
        commodity: commodity.clone(),
        actual_formatted: format_budget_amount(actual, &commodity, &raw_bal),
        budget_formatted: format_budget_amount(budget, &commodity, &raw_bal),
        remaining_formatted: format_budget_amount(remaining, &commodity, &raw_bal),
        tint,
    }
}

fn parse_budget_json(stdout: &str) -> Result<BudgetReport, String> {
    let root: serde_json::Value = serde_json::from_str(stdout).map_err(|e| {
        to_error_string_with_details(
            "budget_failed",
            "Unable to parse budget report output.",
            e.to_string(),
        )
    })?;

    let dates = root["prDates"]
        .as_array()
        .ok_or_else(|| {
            to_error_string_with_details(
                "budget_failed",
                "Unable to parse budget report.",
                "Missing prDates in JSON output.",
            )
        })?;
    let period_columns: Vec<String> = dates.iter().map(period_label).collect();

    let empty_rows = Vec::new();
    let rows_raw = root["prRows"].as_array().unwrap_or(&empty_rows);

    let mut rows: Vec<BudgetRow> = Vec::new();

    for row in rows_raw {
        let account = row["prrName"].as_str().unwrap_or("");
        if account.is_empty() || account == "<unbudgeted>" {
            continue;
        }

        if AMOUNT_STYLE.get().is_none() {
            if let Some(periods) = row["prrAmounts"].as_array() {
                for period in periods {
                    if let Some(sides) = period.as_array() {
                        for side in sides {
                            if let Some(amounts) = side.as_array() {
                                if let Some(first_amt) = amounts.first() {
                                    let style = AmountStyle::from_hledger_json(first_amt);
                                    let _ = AMOUNT_STYLE.set(style);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // prrAmounts structure: [period_data_0, period_data_1, ...]
        // Where each period_data = [[actual_amounts], [budget_amounts]]
        let prr_data = row["prrAmounts"].as_array();

        let mut periods: Vec<BudgetPeriodAmount> = Vec::new();

        for (i, col) in period_columns.iter().enumerate() {
            let period_data = prr_data.and_then(|d| d.get(i)).and_then(|d| d.as_array());

            let actual_data = period_data
                .and_then(|d| d.first())
                .and_then(|d| d.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[]);
            let budget_data = period_data
                .and_then(|d| d.get(1))
                .and_then(|d| d.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[]);

            let actual_amounts = extract_period_amounts(actual_data, None);
            let budget_amounts = extract_period_amounts(budget_data, None);

            periods.push(aggregate_period(col, &actual_amounts, &budget_amounts));
        }

        // Compute totals from periods
        let total_actual: f64 = periods.iter().map(|p| p.actual).sum();
        let total_budget: f64 = periods.iter().map(|p| p.budget).sum();
        let total_remaining = total_budget - total_actual;
        let total_pct_used = if total_budget != 0.0 {
            ((total_actual / total_budget) * 100.0).clamp(-9999.0, 9999.0)
        } else {
            f64::NAN
        };

        let commodity = periods
            .iter()
            .find(|p| !p.commodity.is_empty())
            .map(|p| p.commodity.clone())
            .unwrap_or_default();

        let total_tint = if total_pct_used.is_nan() {
            "neutral"
        } else if total_pct_used > 100.0 {
            "negative"
        } else if total_pct_used > 80.0 {
            "neutral"
        } else {
            "positive"
        };

        let raw_bal = serde_json::Value::Null;

        rows.push(BudgetRow {
            account: account.to_string(),
            periods,
            total_actual,
            total_budget,
            total_remaining,
            total_pct_used,
            commodity: commodity.clone(),
            total_actual_formatted: format_budget_amount(total_actual, &commodity, &raw_bal),
            total_budget_formatted: format_budget_amount(total_budget, &commodity, &raw_bal),
            total_remaining_formatted: format_budget_amount(total_remaining, &commodity, &raw_bal),
            tint: total_tint.to_string(),
        });
    }

    rows.sort_by(|a, b| a.account.cmp(&b.account));

    Ok(BudgetReport {
        period_columns,
        rows,
    })
}

#[tauri::command]
pub(crate) async fn run_budget_report(
    app: AppHandle,
    interval: String,
    scope: String,
    begin_date: Option<String>,
    end_date: Option<String>,
) -> Result<BudgetReport, String> {
    reports::validate_grouping(&interval)?;

    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let executable = hledger_executable(&settings);
    let date_range = reports::resolve_report_date_range(
        &scope,
        begin_date.as_deref(),
        end_date.as_deref(),
        Local::now().date_naive(),
    )?;
    let args = hledger_budget_args(&journal_path, &interval, &date_range);

    logs::log_event(
        &app,
        "info",
        "budget_command_start",
        format!(
            "Starting hledger budget report: executable='{}', args={:?}.",
            executable, args
        ),
    );

    let mut cmd = Command::new(&executable);
    cmd.args(&args);

    let output = run_command_with_timeout(cmd, REPORT_TIMEOUT).inspect_err(|error| {
        logs::log_error(
            &app,
            "budget_failed",
            "hledger budget command did not complete.",
            error,
        );
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        logs::log_error(&app, "budget_failed", "hledger budget command failed.", &stderr);
        return Err(to_error_string_with_details(
            "budget_failed",
            "hledger budget command failed.",
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_budget_json(&stdout)
}

fn hledger_budget_args(
    journal_path: &std::path::Path,
    interval: &str,
    date_range: &ReportDateRange,
) -> Vec<String> {
    let mut args = vec![
        "-f".to_string(),
        journal_path.display().to_string(),
        "balance".to_string(),
        "--budget".to_string(),
    ];

    if let Some(begin) = &date_range.begin {
        args.extend(["-b".to_string(), begin.clone()]);
    }
    if let Some(end) = &date_range.end {
        args.extend(["-e".to_string(), end.clone()]);
    }
    if !interval.is_empty() {
        args.push(interval.to_string());
    }

    args.extend(["-O".to_string(), "json".to_string()]);

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_budget_json_with_periodic_rules() {
        // prrAmounts structure: [period_0, period_1] where each period = [[actuals], [budgets]]
        let json = r#"{
            "prDates": [
                [{"contents": "2026-01-01", "tag": "Exact"}, {"contents": "2026-02-01", "tag": "Exact"}],
                [{"contents": "2026-02-01", "tag": "Exact"}, {"contents": "2026-03-01", "tag": "Exact"}]
            ],
            "prRows": [
                {
                    "prrName": "expenses:food",
                    "prrAmounts": [
                        [
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 86.45}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}],
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 200.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}]
                        ],
                        [
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 210.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}],
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 200.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}]
                        ]
                    ]
                },
                {
                    "prrName": "expenses:rent",
                    "prrAmounts": [
                        [
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 1250.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}],
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 1300.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}]
                        ],
                        [
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 1250.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}],
                            [{"acommodity": "€", "aquantity": {"floatingPoint": 1300.0}, "astyle": {"ascommodityside": "L", "ascommodityspaced": false, "asdecimalmark": ".", "asdigitgroups": [",", [3]], "asprecision": 2}}]
                        ]
                    ]
                },
                {
                    "prrName": "<unbudgeted>",
                    "prrAmounts": null
                }
            ],
            "prTotals": {}
        }"#;

        let result = parse_budget_json(json).expect("budget JSON should parse");
        assert_eq!(result.period_columns, vec!["2026-01", "2026-02"]);
        assert_eq!(result.rows.len(), 2, "should skip <unbudgeted> row");
        let food = result.rows.iter().find(|r| r.account == "expenses:food").expect("food row should exist");
        assert_eq!(food.account, "expenses:food");

        // Jan: actual €86.45, budget €200.00
        let jan = &food.periods[0];
        assert_eq!(jan.actual, 86.45);
        assert_eq!(jan.budget, 200.0);
        assert_eq!(jan.remaining, 113.55);
        assert!((jan.pct_used - 43.225).abs() < 0.01);

        // Feb: actual €210.00, budget €200.00 (over budget)
        let feb = &food.periods[1];
        assert_eq!(feb.actual, 210.0);
        assert_eq!(feb.budget, 200.0);
        assert_eq!(feb.remaining, -10.0);
        assert!((feb.pct_used - 105.0).abs() < 0.01);

        // Totals
        assert_eq!(food.total_actual, 296.45);
        assert_eq!(food.total_budget, 400.0);
        assert!((food.total_pct_used - 74.1125).abs() < 0.01);

        // Rent row
        let rent = result.rows.iter().find(|r| r.account == "expenses:rent").expect("rent row should exist");
        assert_eq!(rent.account, "expenses:rent");
        assert_eq!(rent.periods[0].actual, 1250.0);
        assert_eq!(rent.periods[0].budget, 1300.0);
    }

    #[test]
    fn period_label_formats_dates() {
        let date = serde_json::json!([
            {"contents": "2026-01-01", "tag": "Exact"},
            {"contents": "2026-02-01", "tag": "Exact"}
        ]);
        assert_eq!(period_label(&date), "2026-01");
    }
}
