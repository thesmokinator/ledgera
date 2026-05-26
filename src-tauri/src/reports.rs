use crate::{
    amount_style::{format_hledger_display_amount, AmountStyle},
    app_error::to_error_string_with_details,
    hledger::hledger_executable,
    journal::files::require_journal_path,
    logs,
    settings::read_settings,
    AMOUNT_STYLE,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

const REPORT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportPeriodAmount {
    pub period: String,
    pub amount: f64,
    pub commodity: String,
    pub formatted: String,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportRow {
    pub account: String,
    pub indent: u32,
    pub is_total: bool,
    pub amounts: Vec<ReportPeriodAmount>,
    pub total: ReportPeriodAmount,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportChartEntry {
    pub account: String,
    pub label: String,
    pub amount: f64,
    pub chart_amount: f64,
    pub chart_amount_formatted: String,
    pub commodity: String,
    pub formatted: String,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportPeriodSummary {
    pub period: String,
    pub amount: f64,
    pub chart_amount: f64,
    pub chart_amount_formatted: String,
    pub commodity: String,
    pub formatted: String,
    pub tint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportVisualization {
    pub kind: String,
    pub entries: Vec<ReportChartEntry>,
    pub periods: Vec<ReportPeriodSummary>,
    pub account_level: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportResult {
    pub report_type: String,
    pub interval: String,
    pub period_columns: Vec<String>,
    pub rows: Vec<ReportRow>,
    pub visualization: ReportVisualization,
}

fn tint(amount: f64) -> String {
    if amount < 0.0 {
        "negative".to_string()
    } else if amount > 0.0 {
        "positive".to_string()
    } else {
        "neutral".to_string()
    }
}

fn report_visualization_kind(report_type: &str) -> String {
    match report_type {
        "bs" => "allocation".to_string(),
        "is" => "breakdown".to_string(),
        "cf" => "cashflow".to_string(),
        _ => "breakdown".to_string(),
    }
}

fn account_at_level(account: &str, level: u32) -> String {
    if level == 0 {
        return account.to_string();
    }

    let parts: Vec<&str> = account.split(':').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return account.to_string();
    }

    parts
        .iter()
        .take(level as usize)
        .copied()
        .collect::<Vec<_>>()
        .join(":")
}

fn account_display_label(account: &str) -> String {
    account
        .split(':')
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(account)
        .to_string()
}

#[derive(Default)]
struct AggregatedAmount {
    amount: f64,
    commodity: String,
    mixed_commodities: bool,
}

fn merge_commodity(current: &mut AggregatedAmount, commodity: &str) {
    if commodity.is_empty() {
        return;
    }

    if current.commodity.is_empty() {
        current.commodity = commodity.to_string();
    } else if current.commodity != commodity {
        current.mixed_commodities = true;
        current.commodity.clear();
    }
}

fn aggregate_report_entries(rows: &[ReportRow], account_level: u32) -> Vec<ReportChartEntry> {
    let mut grouped: HashMap<String, AggregatedAmount> = HashMap::new();

    for row in rows.iter().filter(|row| !row.is_total) {
        let account = account_at_level(&row.account, account_level);
        let entry = grouped.entry(account).or_default();
        entry.amount += row.total.amount;
        merge_commodity(entry, &row.total.commodity);
    }

    let mut entries: Vec<ReportChartEntry> = grouped
        .into_iter()
        .map(|(account, aggregated)| {
            let commodity = if aggregated.mixed_commodities {
                String::new()
            } else {
                aggregated.commodity
            };
            let chart_amount = aggregated.amount.abs();
            ReportChartEntry {
                label: account_display_label(&account),
                account,
                amount: aggregated.amount,
                chart_amount,
                chart_amount_formatted: format_amount(
                    chart_amount,
                    &commodity,
                    &serde_json::Value::Null,
                ),
                formatted: format_amount(aggregated.amount, &commodity, &serde_json::Value::Null),
                commodity,
                tint: tint(aggregated.amount),
            }
        })
        .filter(|entry| entry.chart_amount > 0.0)
        .collect();

    entries.sort_by(|a, b| b.chart_amount.total_cmp(&a.chart_amount));
    entries
}

fn aggregate_report_periods(
    period_columns: &[String],
    rows: &[ReportRow],
) -> Vec<ReportPeriodSummary> {
    period_columns
        .iter()
        .map(|period| {
            let mut aggregated = AggregatedAmount::default();
            for row in rows.iter().filter(|row| !row.is_total) {
                if let Some(amount) = row.amounts.iter().find(|amount| &amount.period == period) {
                    aggregated.amount += amount.amount;
                    merge_commodity(&mut aggregated, &amount.commodity);
                }
            }

            let commodity = if aggregated.mixed_commodities {
                String::new()
            } else {
                aggregated.commodity
            };
            let chart_amount = aggregated.amount.abs();
            ReportPeriodSummary {
                period: period.clone(),
                amount: aggregated.amount,
                chart_amount,
                chart_amount_formatted: format_amount(
                    chart_amount,
                    &commodity,
                    &serde_json::Value::Null,
                ),
                formatted: format_amount(aggregated.amount, &commodity, &serde_json::Value::Null),
                commodity,
                tint: tint(aggregated.amount),
            }
        })
        .collect()
}

const MIN_USEFUL_BALANCE_SHEET_GROUPS: usize = 4;
const MAX_USEFUL_BALANCE_SHEET_GROUPS: usize = 10;

fn account_depth(account: &str) -> u32 {
    account
        .split(':')
        .filter(|part| !part.is_empty())
        .count()
        .max(1) as u32
}

fn max_account_depth(rows: &[ReportRow]) -> u32 {
    rows.iter()
        .filter(|row| !row.is_total && row.total.amount != 0.0)
        .map(|row| account_depth(&row.account))
        .max()
        .unwrap_or(2)
}

fn group_count_at_level(rows: &[ReportRow], level: u32) -> usize {
    rows.iter()
        .filter(|row| !row.is_total && row.total.amount != 0.0)
        .map(|row| account_at_level(&row.account, level))
        .collect::<HashSet<_>>()
        .len()
}

fn choose_balance_sheet_account_level(rows: &[ReportRow]) -> u32 {
    let max_depth = max_account_depth(rows);
    let start_level = 2.min(max_depth).max(1);
    let mut best_level = start_level;

    for level in start_level..=max_depth {
        let group_count = group_count_at_level(rows, level);
        if group_count == 0 {
            continue;
        }

        if group_count > MAX_USEFUL_BALANCE_SHEET_GROUPS {
            return best_level;
        }

        best_level = level;

        if group_count >= MIN_USEFUL_BALANCE_SHEET_GROUPS {
            return level;
        }
    }

    best_level
}

fn choose_account_level(report_type: &str, rows: &[ReportRow]) -> u32 {
    match report_type {
        "bs" => choose_balance_sheet_account_level(rows),
        "is" | "cf" => 2,
        _ => 2,
    }
}

fn build_visualization(
    report_type: &str,
    period_columns: &[String],
    rows: &[ReportRow],
) -> ReportVisualization {
    let account_level = choose_account_level(report_type, rows);
    ReportVisualization {
        kind: report_visualization_kind(report_type),
        entries: aggregate_report_entries(rows, account_level),
        periods: aggregate_report_periods(period_columns, rows),
        account_level,
    }
}

fn empty_visualization(report_type: &str) -> ReportVisualization {
    ReportVisualization {
        kind: report_visualization_kind(report_type),
        entries: Vec::new(),
        periods: Vec::new(),
        account_level: choose_account_level(report_type, &[]),
    }
}

fn format_amount(qty: f64, commodity: &str, raw_bal: &serde_json::Value) -> String {
    let effective = if commodity.is_empty() {
        "€"
    } else {
        commodity
    };
    format_hledger_display_amount(qty, effective, raw_bal)
}

fn amount_quantity(raw_amount: &serde_json::Value) -> f64 {
    raw_amount["aquantity"]
        .get("floatingPoint")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

fn amount_commodity(raw_amount: &serde_json::Value) -> String {
    raw_amount["acommodity"].as_str().unwrap_or("€").to_string()
}

fn summarize_amounts(period: String, raw_amounts: &[serde_json::Value]) -> ReportPeriodAmount {
    if raw_amounts.is_empty() {
        return ReportPeriodAmount {
            period,
            amount: 0.0,
            commodity: String::new(),
            formatted: "-".to_string(),
            tint: "neutral".to_string(),
        };
    }

    let mut total = 0.0;
    let mut formatted_parts = Vec::new();
    let mut commodities = Vec::new();
    let mut has_positive = false;
    let mut has_negative = false;

    for raw_amount in raw_amounts {
        let quantity = amount_quantity(raw_amount);
        let commodity = amount_commodity(raw_amount);
        total += quantity;
        if quantity > 0.0 {
            has_positive = true;
        } else if quantity < 0.0 {
            has_negative = true;
        }
        if !commodities.contains(&commodity) {
            commodities.push(commodity.clone());
        }
        formatted_parts.push(format_amount(quantity, &commodity, raw_amount));
    }

    let tint = if has_positive && !has_negative {
        "positive"
    } else if has_negative && !has_positive {
        "negative"
    } else {
        "neutral"
    };

    ReportPeriodAmount {
        period,
        amount: total,
        commodity: if commodities.len() == 1 {
            commodities.remove(0)
        } else {
            String::new()
        },
        formatted: formatted_parts.join(" / "),
        tint: tint.to_string(),
    }
}

fn period_label(period: &serde_json::Value) -> String {
    let start = period
        .as_array()
        .and_then(|dates| dates.first())
        .and_then(|date| date["contents"].as_str())
        .unwrap_or_default();
    let end = period
        .as_array()
        .and_then(|dates| dates.get(1))
        .and_then(|date| date["contents"].as_str())
        .unwrap_or_default();

    if start.len() >= 7 && start.ends_with("-01") {
        start[..7].to_string()
    } else if !start.is_empty() && !end.is_empty() {
        format!("{} → {}", start, end)
    } else {
        start.to_string()
    }
}

fn parse_period_amounts(
    period_columns: &[String],
    raw_period_amounts: Option<&Vec<serde_json::Value>>,
) -> Vec<ReportPeriodAmount> {
    period_columns
        .iter()
        .enumerate()
        .map(|(index, period)| {
            let amounts = raw_period_amounts
                .and_then(|periods| periods.get(index))
                .and_then(|period_amounts| period_amounts.as_array())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            summarize_amounts(period.clone(), amounts)
        })
        .collect()
}

fn parse_compound_report_json(root: &serde_json::Value) -> Option<ReportResult> {
    let dates = root["cbrDates"].as_array()?;
    let period_columns: Vec<String> = dates.iter().map(period_label).collect();
    let mut rows = Vec::new();

    if let Some(subreports) = root["cbrSubreports"].as_array() {
        for subreport in subreports {
            let title = subreport
                .as_array()
                .and_then(|items| items.first())
                .and_then(|value| value.as_str())
                .unwrap_or("Report");
            let report = subreport
                .as_array()
                .and_then(|items| items.get(1))
                .unwrap_or(&serde_json::Value::Null);

            let totals = &report["prTotals"];
            rows.push(ReportRow {
                account: title.to_string(),
                indent: 1,
                is_total: true,
                amounts: parse_period_amounts(&period_columns, totals["prrAmounts"].as_array()),
                total: summarize_amounts(
                    String::new(),
                    totals["prrTotal"]
                        .as_array()
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ),
            });

            if let Some(report_rows) = report["prRows"].as_array() {
                for report_row in report_rows {
                    let account = report_row["prrName"].as_str().unwrap_or_default();
                    if account.is_empty() {
                        continue;
                    }
                    rows.push(ReportRow {
                        account: account.to_string(),
                        indent: 2,
                        is_total: false,
                        amounts: parse_period_amounts(
                            &period_columns,
                            report_row["prrAmounts"].as_array(),
                        ),
                        total: summarize_amounts(
                            String::new(),
                            report_row["prrTotal"]
                                .as_array()
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                        ),
                    });
                }
            }
        }
    }

    let compound_totals = &root["cbrTotals"];
    if !compound_totals.is_null() {
        rows.push(ReportRow {
            account: "Total".to_string(),
            indent: 1,
            is_total: true,
            amounts: parse_period_amounts(
                &period_columns,
                compound_totals["prrAmounts"].as_array(),
            ),
            total: summarize_amounts(
                String::new(),
                compound_totals["prrTotal"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
        });
    }

    Some(ReportResult {
        report_type: String::new(),
        interval: String::new(),
        period_columns,
        rows,
        visualization: empty_visualization(""),
    })
}

fn parse_legacy_account_tree_report_json(raw: &[serde_json::Value]) -> ReportResult {
    let mut period_columns: Vec<String> = Vec::new();
    let mut rows: Vec<ReportRow> = Vec::new();

    fn walk(
        nodes: &[serde_json::Value],
        period_columns: &mut Vec<String>,
        rows: &mut Vec<ReportRow>,
    ) {
        for node in nodes {
            let acctname = node["acctname"].as_str().unwrap_or("").to_string();
            let aindent = node["aindent"].as_u64().unwrap_or(1) as u32;
            let afullwidth = node["afullwidth"].as_bool().unwrap_or(false);

            if acctname.is_empty() {
                if let Some(subs) = node["subaccounts"].as_array() {
                    walk(subs, period_columns, rows);
                }
                continue;
            }

            let balance = node["abalance"].as_array();
            let total_bal = node["aTotal"].as_array();

            // Collect all period labels
            if let Some(balances) = balance {
                for b in balances {
                    let period = b["period"].as_str().unwrap_or("").to_string();
                    if !period.is_empty() && !period_columns.contains(&period) {
                        period_columns.push(period.to_string());
                    }
                }
            }

            // Build period amounts
            let mut amounts: Vec<ReportPeriodAmount> = Vec::new();
            if let Some(balances) = balance {
                for b in balances {
                    let period = b["period"].as_str().unwrap_or("").to_string();
                    let comm = b["acommodity"].as_str().unwrap_or("").to_string();
                    let qty = b["aquantity"]
                        .get("floatingPoint")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    amounts.push(ReportPeriodAmount {
                        period,
                        amount: qty,
                        commodity: comm.clone(),
                        formatted: format_amount(qty, &comm, b),
                        tint: tint(qty),
                    });
                }
            }

            // Build total
            let total = if let Some(totals) = total_bal {
                totals.first().map_or(
                    ReportPeriodAmount {
                        period: String::new(),
                        amount: 0.0,
                        commodity: String::new(),
                        formatted: "0".to_string(),
                        tint: "neutral".to_string(),
                    },
                    |t| {
                        let comm = t["acommodity"].as_str().unwrap_or("").to_string();
                        let qty = t["aquantity"]
                            .get("floatingPoint")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        ReportPeriodAmount {
                            period: String::new(),
                            amount: qty,
                            commodity: comm.clone(),
                            formatted: format_amount(qty, &comm, t),
                            tint: tint(qty),
                        }
                    },
                )
            } else {
                // For rows without aTotal, sum the period amounts
                let qty: f64 = amounts.iter().map(|a| a.amount).sum();
                let comm = amounts
                    .first()
                    .map(|a| a.commodity.clone())
                    .unwrap_or_default();
                ReportPeriodAmount {
                    period: String::new(),
                    amount: qty,
                    commodity: comm.clone(),
                    formatted: format_amount(qty, &comm, &serde_json::Value::Null),
                    tint: tint(qty),
                }
            };

            rows.push(ReportRow {
                account: acctname,
                indent: aindent,
                is_total: afullwidth,
                amounts,
                total,
            });

            if let Some(subs) = node["subaccounts"].as_array() {
                walk(subs, period_columns, rows);
            }
        }
    }

    walk(raw, &mut period_columns, &mut rows);

    ReportResult {
        report_type: String::new(),
        interval: String::new(),
        period_columns,
        rows,
        visualization: empty_visualization(""),
    }
}

fn parse_report_json(app: &AppHandle, stdout: &str) -> Result<ReportResult, String> {
    logs::log_event(
        app,
        "info",
        "report_parse_start",
        format!(
            "Parsing hledger report JSON output ({} bytes).",
            stdout.len()
        ),
    );

    let root: serde_json::Value = serde_json::from_str(stdout).map_err(|e| {
        to_error_string_with_details(
            "report_failed",
            "Unable to parse report output.",
            e.to_string(),
        )
    })?;

    logs::log_event(
        app,
        "info",
        "report_parse_shape",
        match &root {
            serde_json::Value::Object(object) => format!(
                "Report JSON root is an object with keys: {:?}.",
                object.keys().cloned().collect::<Vec<_>>()
            ),
            serde_json::Value::Array(array) => {
                format!("Report JSON root is an array with {} items.", array.len())
            }
            _ => "Report JSON root has an unsupported shape.".to_string(),
        },
    );

    let result = if root.get("cbrSubreports").is_some() || root.get("cbrDates").is_some() {
        parse_compound_report_json(&root).ok_or_else(|| {
            to_error_string_with_details(
                "report_failed",
                "Unable to parse report output.",
                "Compound report JSON did not contain cbrDates.",
            )
        })?
    } else if let Some(raw) = root.as_array() {
        parse_legacy_account_tree_report_json(raw)
    } else {
        return Err(to_error_string_with_details(
            "report_failed",
            "Unable to parse report output.",
            "Unsupported hledger JSON report shape.",
        ));
    };

    logs::log_event(
        app,
        "info",
        "report_parse_done",
        format!(
            "Parsed report JSON: {} rows, {} period columns.",
            result.rows.len(),
            result.period_columns.len()
        ),
    );

    Ok(result)
}

fn read_pipe<R>(mut pipe: R) -> Vec<u8>
where
    R: Read,
{
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    bytes
}

fn join_pipe_reader(handle: thread::JoinHandle<Vec<u8>>) -> Vec<u8> {
    handle.join().unwrap_or_default()
}

fn run_command_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Output, String> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            to_error_string_with_details(
                "report_failed",
                "Unable to run hledger report.",
                error.to_string(),
            )
        })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        to_error_string_with_details(
            "report_failed",
            "Unable to read hledger report output.",
            "stdout pipe was unavailable.",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        to_error_string_with_details(
            "report_failed",
            "Unable to read hledger report output.",
            "stderr pipe was unavailable.",
        )
    })?;
    let stdout_reader = thread::spawn(move || read_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_pipe(stderr));

    let started_at = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            to_error_string_with_details(
                "report_failed",
                "Unable to run hledger report.",
                error.to_string(),
            )
        })? {
            return Ok(Output {
                status,
                stdout: join_pipe_reader(stdout_reader),
                stderr: join_pipe_reader(stderr_reader),
            });
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(to_error_string_with_details(
                "report_failed",
                "hledger report timed out.",
                format!("Report generation exceeded {} seconds.", timeout.as_secs()),
            ));
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn run_hledger_report(
    app: &AppHandle,
    report_type: &str,
    interval: &str,
) -> Result<ReportResult, String> {
    let settings = read_settings(app)?;
    let journal_path = require_journal_path(&settings)?;
    let executable = hledger_executable(&settings);

    let mut args = vec![
        "-f".to_string(),
        journal_path.display().to_string(),
        report_type.to_string(),
    ];

    if !interval.is_empty() {
        args.push(interval.to_string());
    }

    // Use --valuechange for balance sheet, which is the default for bs/cf
    // For is, hledger already shows period changes
    if report_type == "bs" || report_type == "cf" {
        args.push("--valuechange".to_string());
    }

    args.extend([
        "-V".to_string(),
        "--infer-market-prices".to_string(),
        "-O".to_string(),
        "json".to_string(),
    ]);

    logs::log_event(
        app,
        "info",
        "report_command_start",
        format!(
            "Starting hledger report: executable='{}', args={:?}, timeout={}s.",
            executable,
            args,
            REPORT_TIMEOUT.as_secs()
        ),
    );

    let started_at = Instant::now();
    let mut cmd = Command::new(&executable);
    cmd.args(&args);

    let output = run_command_with_timeout(cmd, REPORT_TIMEOUT).map_err(|error| {
        logs::log_error(
            app,
            "report_failed",
            "hledger report command did not complete.",
            &error,
        );
        error
    })?;

    logs::log_event(
        app,
        "info",
        "report_command_done",
        format!(
            "hledger report finished in {}ms: success={}, code={:?}, stdout={} bytes, stderr={} bytes.",
            started_at.elapsed().as_millis(),
            output.status.success(),
            output.status.code(),
            output.stdout.len(),
            output.stderr.len()
        ),
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        logs::log_error(
            app,
            "report_failed",
            "hledger report command failed.",
            stderr.clone(),
        );
        return Err(to_error_string_with_details(
            "report_failed",
            "hledger report command failed.",
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Initialize AMOUNT_STYLE from hledger output if not already set
    if AMOUNT_STYLE.get().is_none() {
        if let Ok(raw) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
            for node in &raw {
                if let Some(balances) = node["abalance"].as_array() {
                    if let Some(first) = balances.first() {
                        let style = AmountStyle::from_hledger_json(first);
                        let _ = AMOUNT_STYLE.set(style);
                        break;
                    }
                }
            }
        }
    }

    let mut result = parse_report_json(app, &stdout)?;
    result.report_type = report_type.to_string();
    result.interval = interval.to_string();
    result.visualization = build_visualization(report_type, &result.period_columns, &result.rows);
    Ok(result)
}

#[tauri::command]
pub(crate) async fn run_report(
    app: AppHandle,
    report_type: String,
    interval: String,
) -> Result<ReportResult, String> {
    logs::log_event(
        &app,
        "info",
        "report_request_received",
        format!(
            "Report request received: report_type='{}', interval='{}'.",
            report_type, interval
        ),
    );

    let app_for_task = app.clone();
    let report_type_for_task = report_type.clone();
    let interval_for_task = interval.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        run_hledger_report(&app_for_task, &report_type_for_task, &interval_for_task)
    })
    .await
    .map_err(|error| {
        let error = to_error_string_with_details(
            "report_failed",
            "Unable to run hledger report.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "report_failed",
            "Report blocking task failed.",
            &error,
        );
        error
    })??;

    logs::log_event(
        &app,
        "info",
        "report_request_done",
        format!(
            "Report request completed: report_type='{}', interval='{}', rows={}, period_columns={}.",
            result.report_type,
            result.interval,
            result.rows.len(),
            result.period_columns.len()
        ),
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(period: &str, value: f64, commodity: &str) -> ReportPeriodAmount {
        ReportPeriodAmount {
            period: period.to_string(),
            amount: value,
            commodity: commodity.to_string(),
            formatted: format!("{} {}", value, commodity),
            tint: tint(value),
        }
    }

    fn row(account: &str, total: f64, amounts: Vec<ReportPeriodAmount>) -> ReportRow {
        ReportRow {
            account: account.to_string(),
            indent: 1,
            is_total: false,
            amounts,
            total: ReportPeriodAmount {
                period: String::new(),
                amount: total,
                commodity: "EUR".to_string(),
                formatted: format!("{} EUR", total),
                tint: tint(total),
            },
        }
    }

    fn total_row(account: &str, total: f64) -> ReportRow {
        ReportRow {
            account: account.to_string(),
            indent: 1,
            is_total: true,
            amounts: vec![],
            total: ReportPeriodAmount {
                period: String::new(),
                amount: total,
                commodity: "EUR".to_string(),
                formatted: format!("{} EUR", total),
                tint: tint(total),
            },
        }
    }

    #[test]
    fn account_at_level_preserves_requested_hledger_depth() {
        assert_eq!(account_at_level("expenses:shopping:home", 1), "expenses");
        assert_eq!(
            account_at_level("expenses:shopping:home", 2),
            "expenses:shopping"
        );
        assert_eq!(
            account_at_level("expenses:shopping:home", 3),
            "expenses:shopping:home"
        );
        assert_eq!(
            account_at_level("expenses:shopping:home", 0),
            "expenses:shopping:home"
        );
    }

    #[test]
    fn aggregate_report_entries_groups_non_total_rows_by_account_level() {
        let rows = vec![
            row("expenses:shopping:home", -10.0, vec![]),
            row("expenses:shopping:gifts", -15.0, vec![]),
            row("expenses:groceries", -30.0, vec![]),
            total_row("Expenses", -55.0),
        ];

        let entries = aggregate_report_entries(&rows, 2);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].account, "expenses:groceries");
        assert_eq!(entries[0].amount, -30.0);
        assert_eq!(entries[0].chart_amount, 30.0);
        assert_eq!(entries[0].chart_amount_formatted, "EUR30.00");
        assert_eq!(entries[1].account, "expenses:shopping");
        assert_eq!(entries[1].amount, -25.0);
        assert_eq!(entries[1].chart_amount, 25.0);
        assert_eq!(entries[1].chart_amount_formatted, "EUR25.00");
    }

    #[test]
    fn aggregate_report_periods_sums_each_period_without_total_rows() {
        let rows = vec![
            row(
                "income:salary",
                300.0,
                vec![
                    amount("2026-01", 100.0, "EUR"),
                    amount("2026-02", 200.0, "EUR"),
                ],
            ),
            row(
                "expenses:groceries",
                -120.0,
                vec![
                    amount("2026-01", -50.0, "EUR"),
                    amount("2026-02", -70.0, "EUR"),
                ],
            ),
            total_row("Net", 180.0),
        ];

        let periods = aggregate_report_periods(&["2026-01".into(), "2026-02".into()], &rows);

        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].period, "2026-01");
        assert_eq!(periods[0].amount, 50.0);
        assert_eq!(periods[0].chart_amount, 50.0);
        assert_eq!(periods[0].chart_amount_formatted, "EUR50.00");
        assert_eq!(periods[1].period, "2026-02");
        assert_eq!(periods[1].amount, 130.0);
        assert_eq!(periods[1].chart_amount, 130.0);
        assert_eq!(periods[1].chart_amount_formatted, "EUR130.00");
    }

    #[test]
    fn choose_balance_sheet_account_level_keeps_shallow_ledgers_at_level_2() {
        let rows = vec![
            row("assets:fineco", 100.0, vec![]),
            row("assets:postepay", 50.0, vec![]),
            row("assets:cash", 10.0, vec![]),
        ];

        assert_eq!(choose_balance_sheet_account_level(&rows), 2);
    }

    #[test]
    fn choose_balance_sheet_account_level_drills_into_nested_accounts_when_useful() {
        let rows = vec![
            row("assets:bank:fineco", 100.0, vec![]),
            row("assets:bank:postepay", 50.0, vec![]),
            row("assets:investments:xeon", 1000.0, vec![]),
            row("assets:cash", 10.0, vec![]),
        ];

        assert_eq!(choose_balance_sheet_account_level(&rows), 3);
    }

    #[test]
    fn choose_balance_sheet_account_level_avoids_too_many_groups() {
        let rows = (0..12)
            .map(|index| row(&format!("assets:bank:account{}", index), 10.0, vec![]))
            .collect::<Vec<_>>();

        assert_eq!(choose_balance_sheet_account_level(&rows), 2);
    }

    #[test]
    fn build_visualization_uses_report_specific_kind() {
        let rows = vec![row(
            "assets:bank:fineco",
            100.0,
            vec![amount("2026", 100.0, "EUR")],
        )];
        let periods = vec!["2026".to_string()];

        let bs = build_visualization("bs", &periods, &rows);
        let is = build_visualization("is", &periods, &rows);
        let cf = build_visualization("cf", &periods, &rows);

        assert_eq!(bs.kind, "allocation");
        assert_eq!(is.kind, "breakdown");
        assert_eq!(cf.kind, "cashflow");
        assert_eq!(bs.account_level, 3);
        assert_eq!(is.account_level, 2);
        assert_eq!(cf.account_level, 2);
        assert_eq!(bs.entries[0].account, "assets:bank:fineco");
    }
}
