use crate::{
    amount_style::{parse_amount_style, parse_commodity_styles},
    journal::{
        autocomplete::collect_declared_commodities,
        files::load_journal_files,
        parser::load_transactions_from_journal_via_files,
        types::{DashboardSummary, JournalSummary, JournalTransaction},
    },
    AMOUNT_STYLE, COMMODITY_STYLES,
};
use chrono::{Datelike, Local, NaiveDate};
use std::path::Path;

pub(crate) fn read_journal_summary(journal_path: &Path) -> Result<JournalSummary, String> {
    let files = load_journal_files(journal_path)?;
    let file_count = files.len();
    let total_size_bytes: u64 = files.iter().map(|f| f.content.len() as u64).sum();

    let amount_style = parse_amount_style(&files, "€");
    let _ = AMOUNT_STYLE.set(amount_style.clone());
    let commodity_styles = parse_commodity_styles(&files);
    let _ = COMMODITY_STYLES.set(commodity_styles);

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
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date <= today)
        .unwrap_or(false)
}

/// Returns whether a transaction is scheduled later in the current month.
fn is_scheduled_this_month(transaction: &JournalTransaction) -> bool {
    let today = Local::now().date_naive();
    parse_journal_date(&transaction.date)
        .map(|date| date.year() == today.year() && date.month() == today.month() && date > today)
        .unwrap_or(false)
}

fn parse_journal_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}
