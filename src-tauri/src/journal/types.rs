use crate::{amount_style::AmountStyle, journal::autocomplete::AutocompleteSuggestions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalSummary {
    pub(crate) path: String,
    pub(crate) transactions: Vec<JournalTransaction>,
    pub(crate) commodities: Vec<String>,
    pub(crate) suggestions: AutocompleteSuggestions,
    pub(crate) file_count: usize,
    pub(crate) total_size_bytes: u64,
    pub(crate) amount_style: AmountStyle,
    pub(crate) dashboard: DashboardSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardSummary {
    pub(crate) monthly_transactions: Vec<JournalTransaction>,
    pub(crate) scheduled_transactions: Vec<JournalTransaction>,
    pub(crate) active_accounts_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalTransaction {
    pub(crate) id: String,
    pub(crate) source_file: String,
    pub(crate) date: String,
    pub(crate) status: String,
    pub(crate) code: String,
    pub(crate) description: String,
    pub(crate) postings: Vec<JournalPosting>,
    pub(crate) display: TransactionDisplay,
    pub(crate) raw: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionDisplay {
    pub(crate) account: String,
    pub(crate) amount: String,
    pub(crate) formatted: String,
    pub(crate) kind: String,
    pub(crate) tint: String,
    pub(crate) flow: TransactionFlow,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionFlow {
    pub(crate) from: Vec<String>,
    pub(crate) to: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalPosting {
    pub(crate) account: String,
    pub(crate) amount: String,
    pub(crate) commodity: String,
    pub(crate) unit_price: String,
    pub(crate) comment: String,
    pub(crate) raw: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionInput {
    #[serde(default)]
    pub(crate) mode: String,
    pub(crate) date: String,
    pub(crate) status: String,
    pub(crate) code: String,
    pub(crate) description: String,
    pub(crate) postings: Vec<PostingInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PostingInput {
    pub(crate) account: String,
    #[serde(default)]
    pub(crate) amount: String,
    #[serde(default)]
    pub(crate) commodity: String,
    #[serde(default)]
    pub(crate) unit_price: String,
    #[serde(default)]
    pub(crate) comment: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TransactionBlock {
    pub(crate) transaction: JournalTransaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RoutingStrategy {
    Glob(Vec<String>),
    Flat(Vec<String>),
    Fallback,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeriodicRule {
    pub(crate) id: String,
    pub(crate) rule_id: String,
    pub(crate) source_file: String,
    pub(crate) period_expr: String,
    pub(crate) description: String,
    pub(crate) postings: Vec<JournalPosting>,
    pub(crate) status: String,
    pub(crate) code: String,
    pub(crate) start_date: Option<String>,
    pub(crate) end_date: Option<String>,
    pub(crate) comment: String,
    pub(crate) raw: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeriodicRulesSummary {
    pub(crate) rules: Vec<PeriodicRule>,
    pub(crate) rule_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeriodicRuleInput {
    pub(crate) rule_id: String,
    pub(crate) period_expr: String,
    pub(crate) description: String,
    pub(crate) postings: Vec<PostingInput>,
    #[allow(dead_code)]
    pub(crate) status: String,
    #[allow(dead_code)]
    pub(crate) code: String,
    #[serde(default)]
    pub(crate) start_date: Option<String>,
    #[serde(default)]
    pub(crate) end_date: Option<String>,
    #[serde(default)]
    pub(crate) comment: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingRecurringDates {
    pub(crate) rule_id: String,
    pub(crate) description: String,
    pub(crate) dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateResult {
    pub(crate) generated: usize,
    pub(crate) rules: Vec<String>,
}
