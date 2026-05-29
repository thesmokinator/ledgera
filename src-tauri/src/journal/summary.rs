use crate::{
    amount_style::{parse_amount_style, parse_commodity_styles},
    journal::{
        autocomplete::{
            build_journal_profile, collect_declared_commodities, unique_sorted,
            AutocompleteSuggestions,
        },
        files::load_journal_files,
        parser::load_transactions_from_journal_via_files,
        types::{DashboardSummary, JournalSummary, JournalTransaction},
        util::parse_journal_date,
    },
    COMMODITY_STYLES,
};
use chrono::{Datelike, Local, NaiveDate};
use std::path::Path;

pub(crate) fn read_journal_summary(
    journal_path: &Path,
    default_commodity: &str,
) -> Result<JournalSummary, String> {
    let files = load_journal_files(journal_path)?;
    let file_count = files.len();
    let total_size_bytes: u64 = files.iter().map(|f| f.content.len() as u64).sum();

    let amount_style = parse_amount_style(&files);
    let commodity_styles = parse_commodity_styles(&files);
    let _ = COMMODITY_STYLES.set(commodity_styles);

    let transactions = load_transactions_from_journal_via_files(&files)?;
    let commodities = collect_declared_commodities(&files);
    let dashboard = build_dashboard_summary(&transactions);
    let profile = build_journal_profile(&transactions, default_commodity);

    let suggestions = AutocompleteSuggestions {
        codes: unique_sorted(
            transactions
                .iter()
                .map(|t| t.code.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect(),
        ),
        descriptions: unique_sorted(
            transactions
                .iter()
                .map(|t| t.description.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect(),
        ),
        accounts: unique_sorted(
            transactions
                .iter()
                .flat_map(|t| t.postings.iter())
                .map(|p| p.account.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect(),
        ),
        commodities: commodities.clone(),
        comments: unique_sorted(
            transactions
                .iter()
                .flat_map(|t| t.postings.iter())
                .map(|p| p.comment.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect(),
        ),
        default_commodity: default_commodity.to_string(),
        default_cash_account: profile.default_cash_account,
        default_expense_account: profile.default_expense_account,
        default_income_account: profile.default_income_account,
        default_transfer_account: profile.default_transfer_account,
        default_investment_account: profile.default_investment_account,
        default_investment_commodity: profile.default_investment_commodity,
    };

    Ok(JournalSummary {
        path: journal_path.to_string_lossy().to_string(),
        transactions,
        commodities,
        suggestions,
        file_count,
        total_size_bytes,
        amount_style,
        dashboard,
    })
}

/// Builds dashboard-specific transaction groups.
pub(crate) fn build_dashboard_summary(transactions: &[JournalTransaction]) -> DashboardSummary {
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
    is_current_month(transaction, |date, today| date <= today)
}

/// Returns whether a transaction is scheduled later in the current month.
fn is_scheduled_this_month(transaction: &JournalTransaction) -> bool {
    is_current_month(transaction, |date, today| date > today)
}

fn is_current_month(
    transaction: &JournalTransaction,
    cmp: impl Fn(NaiveDate, NaiveDate) -> bool,
) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date).map_or(false, |date| {
        date.year() == today.year() && date.month() == today.month() && cmp(date, today)
    })
}
