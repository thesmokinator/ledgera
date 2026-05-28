use crate::{
    app_error::to_error_string_with_details,
    balances::{load_balances_for_settings, Balance},
    journal::{
        files::require_journal_path, summary::read_journal_summary, types::JournalTransaction,
        util::parse_journal_date,
    },
    settings::read_settings,
};
use chrono::{Datelike, Local};
use serde::Serialize;
use std::collections::HashMap;
use tauri::AppHandle;

#[tauri::command]
pub(crate) async fn get_accounts_overview(
    app: AppHandle,
    activity_range: String,
) -> Result<AccountsOverview, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let summary = read_journal_summary(&journal_path, settings.default_commodity.trim())?;
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountsOverview {
    groups: Vec<AccountOverviewGroup>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountOverviewGroup {
    group: String,
    accounts: Vec<AccountOverviewRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountOverviewRow {
    account: String,
    balance: Option<Balance>,
    activity_count: usize,
    transactions: Vec<JournalTransaction>,
}

fn is_in_account_activity_range(transaction: &JournalTransaction, range: &str) -> bool {
    let date = match parse_journal_date(&transaction.date) {
        Some(date) => date,
        None => return false,
    };
    let today = Local::now().date_naive();

    if range == "current-month" {
        return date.year() == today.year() && date.month() == today.month();
    }

    let days = range.parse::<i64>().unwrap_or(30).max(1);
    let range_start = today - chrono::Duration::days(days - 1);
    date >= range_start && date <= today
}

fn account_group(account: &str) -> &'static str {
    let root = account
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    match root.as_str() {
        "asset" | "assets" => "assets",
        "liability" | "liabilities" | "debt" | "debts" => "liabilities",
        "equity" => "equity",
        "income" | "revenue" | "revenues" => "income",
        "expense" | "expenses" => "expenses",
        _ => "other",
    }
}

fn transaction_includes_account(transaction: &JournalTransaction, account: &str) -> bool {
    transaction
        .postings
        .iter()
        .any(|posting| posting.account.trim().eq_ignore_ascii_case(account))
}

pub(crate) fn build_accounts_overview(
    transactions: &[JournalTransaction],
    balances: Vec<Balance>,
    activity_range: &str,
) -> AccountsOverview {
    let mut account_names = HashMap::<String, String>::new();

    for transaction in transactions {
        for posting in &transaction.postings {
            let account = posting.account.trim();
            if !account.is_empty() {
                account_names
                    .entry(account.to_lowercase())
                    .or_insert_with(|| account.to_string());
            }
        }
    }

    for balance in &balances {
        let account = balance.account.trim();
        if !account.is_empty() {
            account_names
                .entry(account.to_lowercase())
                .or_insert_with(|| account.to_string());
        }
    }

    let visible_transactions = transactions
        .iter()
        .filter(|transaction| is_in_account_activity_range(transaction, activity_range))
        .collect::<Vec<_>>();

    let balances_by_account = balances
        .into_iter()
        .map(|balance| (balance.account.to_lowercase(), balance))
        .collect::<HashMap<_, _>>();

    let mut grouped = HashMap::<&'static str, Vec<AccountOverviewRow>>::new();
    for (account_key, account) in account_names {
        let account_transactions = visible_transactions
            .iter()
            .filter(|transaction| transaction_includes_account(transaction, &account))
            .map(|transaction| (*transaction).to_owned())
            .collect::<Vec<_>>();

        let group = account_group(&account);
        grouped.entry(group).or_default().push(AccountOverviewRow {
            account,
            balance: balances_by_account.get(&account_key).cloned(),
            activity_count: account_transactions.len(),
            transactions: account_transactions,
        });
    }

    let group_order = [
        "assets",
        "liabilities",
        "equity",
        "income",
        "expenses",
        "other",
    ];
    let groups = group_order
        .iter()
        .filter_map(|group| {
            let mut accounts = grouped.remove(*group)?;
            accounts.sort_by_key(|row| row.account.to_lowercase());
            Some(AccountOverviewGroup {
                group: (*group).to_string(),
                accounts,
            })
        })
        .collect();

    AccountsOverview { groups }
}
