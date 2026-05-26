use crate::{
    journal::{
        files::{load_journal_files, require_journal_path, JournalFile},
        parser::load_transactions_from_journal_via_files,
        types::JournalTransaction,
    },
    settings::read_settings,
};
use serde::Serialize;
use std::collections::HashMap;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutocompleteSuggestions {
    codes: Vec<String>,
    descriptions: Vec<String>,
    accounts: Vec<String>,
    commodities: Vec<String>,
    comments: Vec<String>,
    default_commodity: String,
    default_cash_account: String,
    default_expense_account: String,
    default_income_account: String,
    default_transfer_account: String,
    default_investment_account: String,
    default_investment_commodity: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct JournalProfile {
    pub(crate) default_cash_account: String,
    pub(crate) default_expense_account: String,
    pub(crate) default_income_account: String,
    pub(crate) default_transfer_account: String,
    pub(crate) default_investment_account: String,
    pub(crate) default_investment_commodity: String,
}

/// Reads distinct values that can be reused while editing transactions.
#[tauri::command]
pub(crate) fn get_autocomplete_suggestions(
    app: AppHandle,
) -> Result<AutocompleteSuggestions, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let files = load_journal_files(&journal_path)?;
    let transactions = load_transactions_from_journal_via_files(&files)?;
    let commodities = collect_declared_commodities(&files);

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
        commodities,
        comments: unique_sorted(
            transactions
                .iter()
                .flat_map(|transaction| transaction.postings.iter())
                .map(|posting| posting.comment.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        ),
        default_commodity: settings.default_commodity.trim().to_string(),
        default_cash_account: profile.default_cash_account,
        default_expense_account: profile.default_expense_account,
        default_income_account: profile.default_income_account,
        default_transfer_account: profile.default_transfer_account,
        default_investment_account: profile.default_investment_account,
        default_investment_commodity: profile.default_investment_commodity,
    })
}

/// Sorts and removes duplicate values.
fn unique_sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort_by_key(|value| value.to_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    values
}

pub(crate) fn collect_declared_commodities(files: &[JournalFile]) -> Vec<String> {
    let mut commodities = files
        .iter()
        .flat_map(|file| file.content.lines())
        .filter_map(parse_commodity_directive)
        .collect::<Vec<_>>();

    commodities.sort();
    commodities.dedup();
    commodities
}

fn parse_commodity_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return None;
    }

    let (directive, rest) = split_first_token(trimmed);
    if directive != "commodity" {
        return None;
    }

    let commodity = split_inline_comment(rest.trim()).0.trim().trim_matches('"');
    if commodity.is_empty() {
        None
    } else {
        Some(commodity.to_string())
    }
}

/// Extracts common account and commodity defaults from journal history.
pub(crate) fn build_journal_profile(
    transactions: &[JournalTransaction],
    default_commodity: &str,
) -> JournalProfile {
    let mut cash_accounts = HashMap::<String, usize>::new();
    let mut expense_accounts = HashMap::<String, usize>::new();
    let mut income_accounts = HashMap::<String, usize>::new();
    let mut transfer_accounts = HashMap::<String, usize>::new();
    let mut investment_accounts = HashMap::<String, usize>::new();
    let mut investment_commodities = HashMap::<String, usize>::new();

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

fn most_frequent(values: HashMap<String, usize>) -> String {
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

fn split_first_token(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(char::is_whitespace) {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

fn split_inline_comment(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(';') {
        (&value[..index], value[index + 1..].trim())
    } else {
        (value, "")
    }
}
