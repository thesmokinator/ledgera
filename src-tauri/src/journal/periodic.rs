use crate::{
    app_error::to_error_string_with_details,
    hledger::hledger_executable,
    journal::{
        files::{parse_include_directive, require_journal_path},
        parser::{
            format_periodic_rule_text, parse_periodic_rules, replace_line_range, split_lines,
        },
        transactions::append_transaction_routed_preserving_quantities,
        types::{
            GenerateResult, PendingRecurringDates, PeriodicRule, PeriodicRuleInput,
            PeriodicRulesSummary, PostingInput, TransactionInput,
        },
    },
    logs,
    settings::{read_settings, AppSettings},
};
use chrono::{Datelike, Local, NaiveDate};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::AppHandle;

const RECURRING_FILENAME: &str = "recurring.journal";

#[tauri::command]
pub(crate) fn list_periodic_rules(app: AppHandle) -> Result<PeriodicRulesSummary, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = journal_path.with_file_name(RECURRING_FILENAME);
    let rules = if recurring_path.exists() {
        let content = fs::read_to_string(&recurring_path).map_err(|error| {
            let err = to_error_string_with_details(
                "journal_read_failed",
                "Unable to read recurring rules file.",
                error.to_string(),
            );
            logs::log_error(
                &app,
                "journal_read_failed",
                "Unable to read recurring rules file.",
                &err,
            );
            err
        })?;
        parse_periodic_rules(&content, &recurring_path)
    } else {
        Vec::new()
    };
    Ok(PeriodicRulesSummary {
        rule_count: rules.len(),
        rules,
    })
}

#[tauri::command]
pub(crate) fn create_periodic_rule(
    app: AppHandle,
    input: PeriodicRuleInput,
) -> Result<PeriodicRulesSummary, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = ensure_recurring_file(&settings, &journal_path).map_err(|e| {
        logs::log_error(
            &app,
            "periodic_rule_operation",
            "Failed to ensure recurring file.",
            &e,
        );
        e
    })?;

    let formatted = format_periodic_rule_text(&input);
    append_to_recurring_file(&settings, &journal_path, &recurring_path, &formatted).map_err(
        |e| {
            logs::log_error(
                &app,
                "periodic_rule_operation",
                "Failed to append to recurring file.",
                &e,
            );
            e
        },
    )?;

    let content = fs::read_to_string(&recurring_path).map_err(|error| {
        let err = to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "journal_read_failed",
            "Unable to read recurring rules file.",
            &err,
        );
        err
    })?;
    let rules = parse_periodic_rules(&content, &recurring_path);
    Ok(PeriodicRulesSummary {
        rule_count: rules.len(),
        rules,
    })
}

#[tauri::command]
pub(crate) fn update_periodic_rule(
    app: AppHandle,
    rule_id_param: String,
    input: PeriodicRuleInput,
) -> Result<PeriodicRulesSummary, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = ensure_recurring_file(&settings, &journal_path).map_err(|e| {
        logs::log_error(
            &app,
            "periodic_rule_operation",
            "Failed to ensure recurring file.",
            &e,
        );
        e
    })?;

    let content = fs::read_to_string(&recurring_path).map_err(|error| {
        let err = to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "journal_read_failed",
            "Unable to read recurring rules file.",
            &err,
        );
        err
    })?;
    let rules = parse_periodic_rules(&content, &recurring_path);
    let target = rules
        .iter()
        .find(|r| r.rule_id == rule_id_param)
        .ok_or_else(|| {
            let err = to_error_string_with_details(
                "periodic_rule_not_found",
                "Periodic rule not found.",
                format!("Rule id: {}", rule_id_param),
            );
            logs::log_error(
                &app,
                "periodic_rule_not_found",
                "Periodic rule not found.",
                &err,
            );
            err
        })?;

    let new_text = format_periodic_rule_text(&input);
    let file_lines = split_lines(&content);
    let updated = replace_line_range(&file_lines, target.start_line, target.end_line, &new_text);

    mutate_recurring_file(&settings, &journal_path, &recurring_path, &updated).map_err(|e| {
        logs::log_error(
            &app,
            "journal_validation_failed",
            "Failed to mutate recurring file.",
            &e,
        );
        e
    })?;

    let new_content = fs::read_to_string(&recurring_path).map_err(|error| {
        let err = to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "journal_read_failed",
            "Unable to read recurring rules file.",
            &err,
        );
        err
    })?;
    let updated_rules = parse_periodic_rules(&new_content, &recurring_path);
    Ok(PeriodicRulesSummary {
        rule_count: updated_rules.len(),
        rules: updated_rules,
    })
}

#[tauri::command]
pub(crate) fn delete_periodic_rule(
    app: AppHandle,
    rule_id_param: String,
) -> Result<PeriodicRulesSummary, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = recurring_path_for(&journal_path);
    if !recurring_path.exists() {
        return Ok(PeriodicRulesSummary {
            rule_count: 0,
            rules: Vec::new(),
        });
    }

    let content = fs::read_to_string(&recurring_path).map_err(|error| {
        let err = to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "journal_read_failed",
            "Unable to read recurring rules file.",
            &err,
        );
        err
    })?;
    let rules = parse_periodic_rules(&content, &recurring_path);
    let target = rules
        .iter()
        .find(|r| r.rule_id == rule_id_param)
        .ok_or_else(|| {
            let err = to_error_string_with_details(
                "periodic_rule_not_found",
                "Periodic rule not found.",
                format!("Rule id: {}", rule_id_param),
            );
            logs::log_error(
                &app,
                "periodic_rule_not_found",
                "Periodic rule not found.",
                &err,
            );
            err
        })?;

    let file_lines = split_lines(&content);
    let updated = replace_line_range(&file_lines, target.start_line, target.end_line, "");

    mutate_recurring_file(&settings, &journal_path, &recurring_path, &updated).map_err(|e| {
        logs::log_error(
            &app,
            "journal_validation_failed",
            "Failed to mutate recurring file.",
            &e,
        );
        e
    })?;

    let new_content = fs::read_to_string(&recurring_path).map_err(|error| {
        let err = to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        );
        logs::log_error(
            &app,
            "journal_read_failed",
            "Unable to read recurring rules file.",
            &err,
        );
        err
    })?;
    let updated_rules = parse_periodic_rules(&new_content, &recurring_path);
    Ok(PeriodicRulesSummary {
        rule_count: updated_rules.len(),
        rules: updated_rules,
    })
}

#[tauri::command]
pub(crate) fn validate_period_expression(app: AppHandle, expr: String) -> Result<bool, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let executable = hledger_executable(&settings);
    let tmp = std::env::temp_dir().join(format!(
        "ledgera-periodic-validate-{}.journal",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    let test_rule = format!(
        "~ {} from 2000-01-01\n    assets:test  1\n    equity:test\n",
        expr.trim()
    );
    fs::write(&tmp, &test_rule).map_err(|e| {
        let err = format!("Failed to write temp journal: {}", e);
        logs::log_error(
            &app,
            "periodic_rule_operation",
            "Failed to write temporary journal file.",
            &err,
        );
        err
    })?;

    let result = Command::new(&executable)
        .arg("print")
        .arg("--forecast=2000-01-01..2000-12-31")
        .arg("-f")
        .arg(&tmp)
        .arg("-O")
        .arg("json")
        .output();

    let _ = fs::remove_file(&tmp);

    match result {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub(crate) fn compute_pending_recurring(
    app: AppHandle,
    rule_id_filter: Option<String>,
) -> Result<Vec<PendingRecurringDates>, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = journal_path.with_file_name(RECURRING_FILENAME);
    let rules = if recurring_path.exists() {
        let content = fs::read_to_string(&recurring_path).map_err(|error| {
            let err = to_error_string_with_details(
                "journal_read_failed",
                "Unable to read recurring rules file.",
                error.to_string(),
            );
            logs::log_error(
                &app,
                "journal_read_failed",
                "Unable to read recurring rules file.",
                &err,
            );
            err
        })?;
        parse_periodic_rules(&content, &recurring_path)
    } else {
        Vec::new()
    };

    let rules: Vec<&PeriodicRule> = if let Some(ref filter) = rule_id_filter {
        rules.iter().filter(|r| r.rule_id == *filter).collect()
    } else {
        rules.iter().collect()
    };

    let today = Local::now().date_naive();
    let executable = hledger_executable(&settings);
    let mut result = Vec::new();

    for rule in rules {
        let pending =
            compute_pending_dates(rule, &journal_path, &executable, today).map_err(|e| {
                logs::log_error(
                    &app,
                    "periodic_rule_operation",
                    &format!(
                        "Failed to compute pending dates for rule '{}'.",
                        rule.rule_id
                    ),
                    &e,
                );
                e
            })?;
        if !pending.is_empty() {
            result.push(PendingRecurringDates {
                rule_id: rule.rule_id.clone(),
                description: rule.description.clone(),
                dates: pending,
            });
        }
    }

    Ok(result)
}

#[tauri::command]
pub(crate) fn generate_recurring_transactions(
    app: AppHandle,
    rule_id_filter: Option<String>,
) -> Result<GenerateResult, String> {
    let settings = read_settings(&app).map_err(|e| {
        logs::log_error(&app, "settings_read_failed", "Failed to read settings.", &e);
        e
    })?;
    let journal_path = require_journal_path(&settings).map_err(|e| {
        logs::log_error(
            &app,
            "journal_path_missing",
            "Journal path not configured.",
            &e,
        );
        e
    })?;
    let recurring_path = journal_path.with_file_name(RECURRING_FILENAME);
    let rules = if recurring_path.exists() {
        let content = fs::read_to_string(&recurring_path).map_err(|error| {
            let err = to_error_string_with_details(
                "journal_read_failed",
                "Unable to read recurring rules file.",
                error.to_string(),
            );
            logs::log_error(
                &app,
                "journal_read_failed",
                "Unable to read recurring rules file.",
                &err,
            );
            err
        })?;
        parse_periodic_rules(&content, &recurring_path)
    } else {
        return Ok(GenerateResult {
            generated: 0,
            rules: Vec::new(),
        });
    };

    let rules: Vec<&PeriodicRule> = if let Some(ref filter) = rule_id_filter {
        rules.iter().filter(|r| r.rule_id == *filter).collect()
    } else {
        rules.iter().collect()
    };

    let today = Local::now().date_naive();
    let executable = hledger_executable(&settings);
    let mut generated = 0usize;
    let mut affected_rules = Vec::new();

    for rule in rules {
        let pending =
            compute_pending_dates(rule, &journal_path, &executable, today).map_err(|e| {
                logs::log_error(
                    &app,
                    "periodic_rule_operation",
                    &format!(
                        "Failed to compute pending dates for rule '{}'.",
                        rule.rule_id
                    ),
                    &e,
                );
                e
            })?;
        if pending.is_empty() {
            continue;
        }
        for date_str in &pending {
            let txn_input = rule_to_transaction_input(rule, date_str);
            append_transaction_routed_preserving_quantities(&settings, &journal_path, &txn_input)
                .map_err(|e| {
                    logs::log_error(
                        &app,
                        "journal_write_failed",
                        &format!(
                            "Failed to append transaction for rule '{}' on date '{}'.",
                            rule.rule_id, date_str
                        ),
                        &e,
                    );
                    e
                })?;
            generated += 1;
        }
        affected_rules.push(rule.rule_id.clone());
    }

    Ok(GenerateResult {
        generated,
        rules: affected_rules,
    })
}

fn ensure_recurring_file(_settings: &AppSettings, main_journal: &Path) -> Result<PathBuf, String> {
    let recurring_path = main_journal.with_file_name(RECURRING_FILENAME);
    if !recurring_path.exists() {
        fs::write(&recurring_path, "").map_err(|error| {
            to_error_string_with_details(
                "journal_write_failed",
                "Unable to create recurring rules file.",
                error.to_string(),
            )
        })?;

        let main_content = fs::read_to_string(main_journal).map_err(|error| {
            to_error_string_with_details(
                "journal_read_failed",
                "Unable to read journal file.",
                error.to_string(),
            )
        })?;
        let has_recurring_include = main_content
            .lines()
            .any(|line| parse_include_directive(line).is_some_and(|inc| inc == RECURRING_FILENAME));
        if !has_recurring_include {
            let updated = format!("include {}\n{}", RECURRING_FILENAME, main_content);
            fs::write(main_journal, &updated).map_err(|error| {
                to_error_string_with_details(
                    "journal_write_failed",
                    "Unable to update journal file.",
                    error.to_string(),
                )
            })?;
        }
    }
    Ok(recurring_path)
}

fn recurring_path_for(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(RECURRING_FILENAME)
}

fn append_to_recurring_file(
    settings: &AppSettings,
    main_journal: &Path,
    recurring_path: &Path,
    text: &str,
) -> Result<(), String> {
    let original = fs::read_to_string(recurring_path).unwrap_or_default();
    let updated = format!("{}{}\n", original.trim_end(), text);
    mutate_recurring_file(settings, main_journal, recurring_path, &updated)
}

fn mutate_recurring_file(
    settings: &AppSettings,
    main_journal: &Path,
    recurring_path: &Path,
    updated: &str,
) -> Result<(), String> {
    let original = fs::read_to_string(recurring_path).map_err(|error| {
        to_error_string_with_details(
            "journal_read_failed",
            "Unable to read recurring rules file.",
            error.to_string(),
        )
    })?;
    fs::write(recurring_path, updated).map_err(|error| {
        to_error_string_with_details(
            "journal_write_failed",
            "Unable to write recurring rules file.",
            error.to_string(),
        )
    })?;
    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::write(recurring_path, original);
        return Err(error);
    }
    Ok(())
}

fn validate_journal(settings: &AppSettings, journal_path: &Path) -> Result<(), String> {
    let executable = hledger_executable(settings);
    let output = Command::new(&executable)
        .arg("-f")
        .arg(journal_path)
        .arg("check")
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(to_error_string_with_details(
                "journal_validation_failed",
                "hledger check failed.",
                stderr,
            ))
        }
        Err(error) => Err(to_error_string_with_details(
            "journal_validation_failed",
            "Unable to validate journal.",
            error.to_string(),
        )),
    }
}

fn rule_to_transaction_input(rule: &PeriodicRule, date_str: &str) -> TransactionInput {
    let comment = format!("rule-id:{}", rule.rule_id);
    TransactionInput {
        mode: String::new(),
        date: date_str.to_string(),
        status: rule.status.clone(),
        code: rule.code.clone(),
        description: rule.description.clone(),
        postings: rule
            .postings
            .iter()
            .map(|p| PostingInput {
                account: p.account.clone(),
                amount: p.amount.clone(),
                commodity: p.commodity.clone(),
                unit_price: p.unit_price.clone(),
                comment: if !p.comment.is_empty() && !comment.is_empty() {
                    format!("{}; {}", comment, p.comment)
                } else if !comment.is_empty() {
                    comment.clone()
                } else {
                    p.comment.clone()
                },
            })
            .collect(),
    }
}

fn compute_pending_dates(
    rule: &PeriodicRule,
    journal_path: &Path,
    hledger_exe: &str,
    today: NaiveDate,
) -> Result<Vec<String>, String> {
    let start_str = match &rule.start_date {
        Some(s) if !s.trim().is_empty() => s.as_str(),
        _ => return Ok(Vec::new()),
    };
    let start = NaiveDate::parse_from_str(start_str.trim(), "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date '{}': {}", start_str, e))?;

    let end_of_month = last_day_of_month(today);
    let mut end_date = end_of_month;
    if let Some(ref end_str) = rule.end_date {
        if !end_str.trim().is_empty() {
            if let Ok(parsed) = NaiveDate::parse_from_str(end_str.trim(), "%Y-%m-%d") {
                end_date = end_date.min(parsed);
            }
        }
    }
    if start > end_date {
        return Ok(Vec::new());
    }

    let all_dates = generate_occurrences(start, &rule.period_expr, end_date, hledger_exe)?;
    let generated_dates = get_generated_dates(hledger_exe, journal_path, &rule.rule_id)?;

    let pending: Vec<String> = all_dates
        .into_iter()
        .filter(|d| !generated_dates.contains(d))
        .collect();
    Ok(pending)
}

fn last_day_of_month(date: NaiveDate) -> NaiveDate {
    if let Some(next_month) = NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1) {
        next_month - chrono::Duration::days(1)
    } else if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .unwrap_or(date)
    } else {
        date
    }
}

fn generate_occurrences(
    start: NaiveDate,
    period: &str,
    end: NaiveDate,
    hledger_exe: &str,
) -> Result<Vec<String>, String> {
    let normalized = period.trim().to_lowercase();
    match normalized.as_str() {
        "daily" => Ok(daily_occurrences(start, end)),
        "weekly" => Ok(interval_occurrences(start, end, 7)),
        "biweekly" => Ok(interval_occurrences(start, end, 14)),
        "monthly" => Ok(monthly_occurrences(start, end, 1)),
        "bimonthly" => Ok(monthly_occurrences(start, end, 2)),
        "quarterly" => Ok(monthly_occurrences(start, end, 3)),
        "yearly" => Ok(yearly_occurrences(start, end)),
        _ => custom_period_occurrences(start, &normalized, end, hledger_exe),
    }
}

fn daily_occurrences(start: NaiveDate, end: NaiveDate) -> Vec<String> {
    let mut dates = Vec::new();
    let mut current = start;
    while current <= end {
        dates.push(format_date(current));
        current += chrono::Duration::days(1);
    }
    dates
}

fn interval_occurrences(start: NaiveDate, end: NaiveDate, days: i64) -> Vec<String> {
    let mut dates = Vec::new();
    let mut current = start;
    while current <= end {
        dates.push(format_date(current));
        current += chrono::Duration::days(days);
    }
    dates
}

fn monthly_occurrences(start: NaiveDate, end: NaiveDate, step: u32) -> Vec<String> {
    let mut dates = Vec::new();
    let canonical_day = start.day();
    let mut current = start;
    while current <= end {
        dates.push(format_date(current));
        current = advance_months(current, step, canonical_day);
    }
    dates
}

fn advance_months(date: NaiveDate, months: u32, canonical_day: u32) -> NaiveDate {
    let target_month = date.month() as i32 + months as i32;
    let year_offset = (target_month - 1) / 12;
    let month = ((target_month - 1) % 12 + 1) as u32;
    let year = (date.year() + year_offset) as i32;

    let max_day = days_in_month(year, month);
    let day = canonical_day.min(max_day);

    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    if let Some(next) = NaiveDate::from_ymd_opt(year, month + 1, 1)
        .or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
    {
        (next - chrono::Duration::days(1)).day()
    } else {
        30
    }
}

fn yearly_occurrences(start: NaiveDate, end: NaiveDate) -> Vec<String> {
    let mut dates = Vec::new();
    let canonical_day = start.day();
    let canonical_month = start.month();
    let mut current = start;
    while current <= end {
        dates.push(format_date(current));
        let max_day = days_in_month(current.year() + 1, canonical_month);
        let day = canonical_day.min(max_day);
        current = NaiveDate::from_ymd_opt(current.year() + 1, canonical_month, day)
            .unwrap_or(current + chrono::Duration::days(366));
    }
    dates
}

fn custom_period_occurrences(
    start: NaiveDate,
    period_expr: &str,
    end: NaiveDate,
    hledger_exe: &str,
) -> Result<Vec<String>, String> {
    let tmp = std::env::temp_dir().join(format!(
        "ledgera-custom-period-{}.journal",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    let test_rule = format!(
        "~ {} from {}\n    assets:test  1\n    equity:test\n",
        period_expr, start
    );
    fs::write(&tmp, &test_rule).map_err(|e| e.to_string())?;

    let forecast_range = format!("{}..{}", start, end);
    let output = Command::new(hledger_exe)
        .arg("print")
        .arg(&format!("--forecast={}", forecast_range))
        .arg("-f")
        .arg(&tmp)
        .arg("-O")
        .arg("json")
        .output()
        .map_err(|e| format!("Failed to run hledger: {}", e))?;

    let _ = fs::remove_file(&tmp);

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap_or_default();
    let mut dates: Vec<String> = parsed
        .iter()
        .filter_map(|entry| entry.get("tdate").and_then(|d| d.as_str()))
        .map(|d| d.to_string())
        .collect();
    dates.sort();
    dates.dedup();
    Ok(dates)
}

fn get_generated_dates(
    hledger_exe: &str,
    journal_path: &Path,
    rule_id: &str,
) -> Result<Vec<String>, String> {
    let output = Command::new(hledger_exe)
        .arg("-f")
        .arg(journal_path)
        .arg("print")
        .arg(&format!("tag:rule-id={}", rule_id))
        .arg("-O")
        .arg("json")
        .output()
        .map_err(|e| {
            to_error_string_with_details(
                "hledger_query_failed",
                "Failed to query generated transactions.",
                e.to_string(),
            )
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap_or_default();
    let mut dates: Vec<String> = parsed
        .iter()
        .filter_map(|entry| entry.get("tdate").and_then(|d| d.as_str()))
        .map(|d| d.to_string())
        .collect();
    dates.sort();
    dates.dedup();
    Ok(dates)
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
