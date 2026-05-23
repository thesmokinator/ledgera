use crate::{
    amount_style::AmountStyle,
    app_error::to_error_string_with_details,
    journal::{
        files::{load_journal_files, JournalFile},
        types::{
            JournalPosting, JournalTransaction, PostingInput, TransactionBlock, TransactionDisplay,
            TransactionFlow, TransactionInput,
        },
    },
    AMOUNT_STYLE,
};
use std::{path::Path, sync::OnceLock};

fn load_transactions_from_journal(journal_path: &Path) -> Result<Vec<JournalTransaction>, String> {
    let files = load_journal_files(journal_path)?;
    load_transactions_from_journal_via_files(&files)
}

pub(crate) fn load_transactions_from_journal_via_files(
    files: &[JournalFile],
) -> Result<Vec<JournalTransaction>, String> {
    let mut transactions: Vec<JournalTransaction> = files
        .iter()
        .flat_map(|file| parse_transactions(&file.content, &file.path))
        .collect();
    transactions.reverse();
    transactions.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(transactions)
}

/// Parses transaction blocks without attempting to reinterpret ledger semantics.
fn parse_transactions(content: &str, source_path: &Path) -> Vec<JournalTransaction> {
    let lines = split_lines(content);
    let mut transactions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_transaction_header(&lines[index]) {
            index += 1;
            continue;
        }

        let start_line = index + 1;
        let mut end_index = index + 1;
        while end_index < lines.len() && !is_transaction_header(&lines[end_index]) {
            end_index += 1;
        }

        let block_lines = &lines[index..end_index];
        let raw = block_lines.join("\n");
        if let Some(transaction) = parse_transaction_block(source_path, start_line, end_index, &raw)
        {
            transactions.push(transaction);
        }
        index = end_index;
    }

    transactions
}

/// Parses one transaction block.
fn parse_transaction_block(
    source_path: &Path,
    start_line: usize,
    end_line: usize,
    raw: &str,
) -> Option<JournalTransaction> {
    let mut lines = raw.lines();
    let header = lines.next()?.trim();
    let (date, rest) = split_first_token(header);
    let mut remaining = rest.trim_start();
    let mut status = String::new();
    let mut code = String::new();

    if remaining.starts_with('*') || remaining.starts_with('!') {
        status = remaining[..1].to_string();
        remaining = remaining[1..].trim_start();
    }

    if remaining.starts_with('(') {
        if let Some(end) = remaining.find(')') {
            code = remaining[..=end].to_string();
            remaining = remaining[end + 1..].trim_start();
        }
    }

    let postings = raw
        .lines()
        .skip(1)
        .filter_map(parse_posting)
        .collect::<Vec<_>>();

    let display = summarize_transaction(&postings);

    let source_file = source_path.to_string_lossy().to_string();
    Some(JournalTransaction {
        id: format!("{}:{}", source_file, start_line),
        source_file,
        date: date.to_string(),
        status,
        code,
        description: remaining.to_string(),
        postings,
        display,
        raw: raw.to_string(),
        start_line,
        end_line,
    })
}

/// Parses one posting line while preserving the raw text.
fn parse_posting(line: &str) -> Option<JournalPosting> {
    if line.trim().is_empty() || !line.starts_with(char::is_whitespace) {
        return None;
    }

    let trimmed = line.trim();
    if trimmed.starts_with(';') {
        return None;
    }

    let (posting_content, comment) = split_inline_comment(trimmed);
    let (account, amount) = split_posting_account_amount(posting_content);
    let (quantity, commodity) = parse_posting_amount(amount);
    Some(JournalPosting {
        account: account.trim().to_string(),
        amount: quantity,
        commodity,
        comment: comment.to_string(),
        raw: line.to_string(),
    })
}

/// Parses an hledger amount into quantity and commodity fields.
fn parse_posting_amount(amount: &str) -> (String, String) {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    let Some(number_start) = trimmed.find(|character: char| character.is_ascii_digit()) else {
        return (trimmed.to_string(), String::new());
    };

    let sign = if trimmed[..number_start].contains('-') {
        "-"
    } else {
        ""
    };
    let number_end = trimmed[number_start..]
        .char_indices()
        .find(|(_, character)| {
            !(character.is_ascii_digit() || *character == '.' || *character == ',')
        })
        .map(|(index, _)| number_start + index)
        .unwrap_or(trimmed.len());

    let quantity = format!("{}{}", sign, &trimmed[number_start..number_end]);
    let prefix_commodity = trimmed[..number_start].trim().trim_matches('-').trim();
    let suffix_commodity = trimmed[number_end..].trim();
    let commodity = if !prefix_commodity.is_empty() {
        prefix_commodity
    } else {
        suffix_commodity
    };

    (quantity, commodity.to_string())
}

/// Splits a posting into account and amount using hledger's common spacing convention.
fn split_posting_account_amount(value: &str) -> (&str, &str) {
    let mut whitespace_start = None;
    let mut whitespace_len = 0;

    for (index, character) in value.char_indices() {
        if character.is_whitespace() {
            if whitespace_start.is_none() {
                whitespace_start = Some(index);
            }
            whitespace_len += character.len_utf8();
            continue;
        }

        if let Some(start) = whitespace_start {
            if whitespace_len >= 2 {
                return (&value[..start], &value[index..]);
            }
        }
        whitespace_start = None;
        whitespace_len = 0;
    }

    (value, "")
}

/// Splits an inline hledger comment from a posting line.
fn split_inline_comment(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(';') {
        (&value[..index], value[index + 1..].trim())
    } else {
        (value, "")
    }
}

/// Splits the first token from a string.
fn split_first_token(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(char::is_whitespace) {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

/// Returns true when a line appears to start a ledger transaction.
fn is_transaction_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return false;
    }

    trimmed
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(false)
}

/// Finds a parsed transaction by id.
pub(crate) fn find_block(journal_path: &Path, id: &str) -> Result<TransactionBlock, String> {
    load_transactions_from_journal(journal_path)?
        .into_iter()
        .find(|transaction| transaction.id == id)
        .map(|transaction| TransactionBlock { transaction })
        .ok_or_else(|| {
            to_error_string_with_details(
                "transaction_not_found",
                "Transaction not found. It may have been deleted or moved.",
                format!("Transaction id: {}", id),
            )
        })
}

/// Splits content into normalized lines for range replacement.
pub(crate) fn split_lines(content: &str) -> Vec<String> {
    content.lines().map(ToString::to_string).collect()
}

/// Replaces a one-based inclusive line range.
pub(crate) fn replace_line_range(
    lines: &[String],
    start_line: usize,
    end_line: usize,
    replacement: &str,
) -> String {
    let mut result = Vec::new();
    let start_index = start_line.saturating_sub(1);
    let end_index = end_line.min(lines.len());

    result.extend_from_slice(&lines[..start_index]);
    if !replacement.trim().is_empty() {
        result.extend(replacement.lines().map(ToString::to_string));
    }
    result.extend_from_slice(&lines[end_index..]);

    let mut content = result.join("\n");
    content.push('\n');
    content
}

/// Formats a transaction from structured form input.
pub(crate) fn format_transaction(input: &TransactionInput) -> String {
    let mut header = input.date.trim().to_string();
    if !input.status.trim().is_empty() {
        header.push(' ');
        header.push_str(input.status.trim());
    }
    if !input.code.trim().is_empty() {
        header.push(' ');
        header.push_str(input.code.trim());
    }
    if !input.description.trim().is_empty() {
        header.push(' ');
        header.push_str(input.description.trim());
    }

    let postings = input
        .postings
        .iter()
        .filter(|posting| !posting.account.trim().is_empty())
        .map(|posting| format_posting(posting))
        .collect::<Vec<_>>();

    std::iter::once(header)
        .chain(postings)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalizes a numeric quantity to two decimals when possible.
fn normalize_quantity(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.replace(',', ".");
    match normalized.parse::<f64>() {
        Ok(val) => {
            let style = AMOUNT_STYLE.get().unwrap_or_else(|| {
                static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
                DEFAULT.get_or_init(AmountStyle::default)
            });
            style.format(val)
        }
        Err(_) => trimmed.to_string(),
    }
}

/// Formats a quantity and commodity into hledger amount syntax.
fn format_posting_amount(amount: &str, commodity: &str) -> String {
    let (quantity, parsed_commodity) = parse_posting_amount(amount);
    let selected_commodity = if parsed_commodity.trim().is_empty() {
        commodity.trim()
    } else {
        parsed_commodity.trim()
    };
    let quantity = normalize_quantity(&quantity);

    if quantity.is_empty() {
        return String::new();
    }
    if selected_commodity.is_empty() {
        return quantity;
    }

    let sign = if quantity.starts_with('-') { "-" } else { "" };
    let absolute_quantity = quantity.trim_start_matches('-');
    if selected_commodity
        .chars()
        .all(|character| character.is_alphabetic())
    {
        format!("{}{} {}", sign, absolute_quantity, selected_commodity)
    } else {
        format!("{}{}{}", sign, selected_commodity, absolute_quantity)
    }
}

/// Formats a posting, including an optional hledger inline comment.
pub(crate) fn format_posting(posting: &PostingInput) -> String {
    let mut amount = format_posting_amount(&posting.amount, &posting.commodity);
    if !posting.unit_price.trim().is_empty() && !amount.trim().is_empty() {
        amount.push_str(" @ ");
        amount.push_str(posting.unit_price.trim());
    }
    let mut line = if amount.trim().is_empty() {
        format!("    {}", posting.account.trim())
    } else {
        format!("    {:<40} {}", posting.account.trim(), amount)
    };

    if !posting.comment.trim().is_empty() {
        line.push_str("  ; ");
        line.push_str(posting.comment.trim().trim_start_matches(';').trim());
    }

    line
}

/// Builds the transaction display fields consumed by the frontend.
pub(crate) fn summarize_transaction(postings: &[JournalPosting]) -> TransactionDisplay {
    let postings_with_amounts = postings
        .iter()
        .filter(|posting| !posting.amount.trim().is_empty())
        .collect::<Vec<_>>();
    let balancing_amount = postings_with_amounts
        .first()
        .map(|posting| format_posting_amount(&posting.amount, &posting.commodity))
        .unwrap_or_default();
    let inferred_values = infer_posting_values(postings);
    let flow = summarize_transaction_flow(postings, &inferred_values);

    if let Some(display_amount) = summarize_asset_transfer_with_expenses(postings, &inferred_values)
    {
        let account = postings
            .iter()
            .zip(inferred_values.iter())
            .find_map(|(posting, value)| {
                if is_asset_or_liability_account(&posting.account)
                    && value.is_some_and(|value| value < 0.0)
                {
                    Some(posting.account.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| display_amount.account.clone());

        return TransactionDisplay {
            account,
            amount: display_amount.amount,
            formatted: display_amount.formatted,
            kind: "transfer".to_string(),
            tint: "negative".to_string(),
            flow,
        };
    }

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("expenses"))
    {
        let display_amount = summarize_kind_amount(postings, &inferred_values, "expense")
            .unwrap_or_else(|| {
                let amount = if posting.amount.trim().is_empty() {
                    balancing_amount
                } else {
                    format_posting_amount(&posting.amount, &posting.commodity)
                };
                DisplayAmount {
                    amount: format_display_amount(&amount, "expense"),
                    formatted: format_amount_styled_with_commodity(&amount),
                }
            });
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: display_amount.amount,
            formatted: display_amount.formatted,
            kind: "expense".to_string(),
            tint: "negative".to_string(),
            flow,
        };
    }

    if let Some(posting) = postings
        .iter()
        .find(|posting| posting.account.to_lowercase().starts_with("income"))
    {
        let display_amount = summarize_kind_amount(postings, &inferred_values, "income")
            .unwrap_or_else(|| {
                let amount = if posting.amount.trim().is_empty() {
                    balancing_amount.clone()
                } else {
                    format_posting_amount(&posting.amount, &posting.commodity)
                };
                DisplayAmount {
                    amount: format_display_amount(&amount, "income"),
                    formatted: format_amount_styled_with_commodity(&amount),
                }
            });
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: display_amount.amount,
            formatted: display_amount.formatted,
            kind: "income".to_string(),
            tint: "positive".to_string(),
            flow,
        };
    }

    if let Some(posting) = postings_with_amounts
        .first()
        .copied()
        .or_else(|| postings.first())
    {
        let display_amount =
            summarize_balanced_amount(postings, &inferred_values).unwrap_or_else(|| {
                let amount = format_posting_amount(&posting.amount, &posting.commodity);
                let formatted = if amount.is_empty() {
                    "-".to_string()
                } else {
                    format_amount_styled_with_commodity(&amount)
                };
                DisplayAmount {
                    amount: if amount.is_empty() {
                        "-".to_string()
                    } else {
                        amount
                    },
                    formatted,
                }
            });
        let kind = if posting.amount.trim().is_empty() {
            "unknown".to_string()
        } else {
            "transfer".to_string()
        };
        let tint = match kind.as_str() {
            "unknown" => "neutral".to_string(),
            _ => {
                let value = parse_amount_value(&posting.amount);
                if value < 0.0 {
                    "negative".to_string()
                } else if value > 0.0 {
                    "positive".to_string()
                } else {
                    "neutral".to_string()
                }
            }
        };
        return TransactionDisplay {
            account: posting.account.clone(),
            amount: display_amount.amount,
            formatted: display_amount.formatted,
            kind,
            tint,
            flow,
        };
    }

    TransactionDisplay {
        account: "-".to_string(),
        amount: "-".to_string(),
        formatted: "-".to_string(),
        kind: "unknown".to_string(),
        tint: "neutral".to_string(),
        flow,
    }
}

#[derive(Debug)]
struct DisplayAmount {
    amount: String,
    formatted: String,
}

#[derive(Debug)]
struct AccountDisplayAmount {
    account: String,
    amount: String,
    formatted: String,
}

fn infer_posting_values(postings: &[JournalPosting]) -> Vec<Option<f64>> {
    let mut values = postings
        .iter()
        .map(|posting| {
            if posting.amount.trim().is_empty() {
                None
            } else {
                Some(parse_amount_value(&posting.amount))
            }
        })
        .collect::<Vec<_>>();

    let missing_indexes = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| if value.is_none() { Some(index) } else { None })
        .collect::<Vec<_>>();

    if missing_indexes.len() == 1 {
        let explicit_total = values.iter().flatten().sum::<f64>();
        values[missing_indexes[0]] = Some(-explicit_total);
    }

    values
}

fn summarize_asset_transfer_with_expenses(
    postings: &[JournalPosting],
    values: &[Option<f64>],
) -> Option<AccountDisplayAmount> {
    let has_expense = postings
        .iter()
        .any(|posting| posting.account.to_lowercase().starts_with("expenses"));
    if !has_expense {
        return None;
    }

    let has_asset_or_liability_source =
        postings.iter().zip(values.iter()).any(|(posting, value)| {
            is_asset_or_liability_account(&posting.account)
                && value.is_some_and(|value| value < 0.0)
        });
    if !has_asset_or_liability_source {
        return None;
    }

    let positive_asset_indexes = postings
        .iter()
        .zip(values.iter())
        .enumerate()
        .filter_map(|(index, (posting, value))| {
            if posting.account.to_lowercase().starts_with("assets")
                && value.is_some_and(|value| value > 0.0)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let display_amount =
        summarize_amount_indexes(postings, values, &positive_asset_indexes, None, true)?;
    let account = positive_asset_indexes
        .first()
        .and_then(|index| postings.get(*index))
        .map(|posting| posting.account.clone())
        .unwrap_or_default();

    Some(AccountDisplayAmount {
        account,
        amount: display_amount.amount,
        formatted: display_amount.formatted,
    })
}

fn is_asset_or_liability_account(account: &str) -> bool {
    let account = account.to_lowercase();
    account.starts_with("assets") || account.starts_with("liabilities")
}

fn summarize_kind_amount(
    postings: &[JournalPosting],
    values: &[Option<f64>],
    kind: &str,
) -> Option<DisplayAmount> {
    let selected_indexes = postings
        .iter()
        .enumerate()
        .filter_map(|(index, posting)| {
            let account = posting.account.to_lowercase();
            let is_selected = match kind {
                "expense" => account.starts_with("expenses"),
                "income" => account.starts_with("income"),
                _ => false,
            };
            if is_selected {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    summarize_amount_indexes(postings, values, &selected_indexes, Some(kind), true)
}

fn summarize_balanced_amount(
    postings: &[JournalPosting],
    values: &[Option<f64>],
) -> Option<DisplayAmount> {
    let positive_indexes = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| match value {
            Some(value) if *value > 0.0 => Some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    let negative_indexes = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| match value {
            Some(value) if *value < 0.0 => Some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    let selected_indexes = if positive_indexes.is_empty() {
        negative_indexes
    } else {
        positive_indexes
    };

    summarize_amount_indexes(postings, values, &selected_indexes, None, true)
}

fn summarize_amount_indexes(
    postings: &[JournalPosting],
    values: &[Option<f64>],
    indexes: &[usize],
    kind: Option<&str>,
    formatted_with_commodity: bool,
) -> Option<DisplayAmount> {
    let mut parts = Vec::<(String, f64)>::new();

    for index in indexes {
        let Some(value) = values.get(*index).and_then(|value| *value) else {
            continue;
        };
        if value == 0.0 {
            continue;
        }
        let commodity = postings
            .get(*index)
            .map(|posting| clean_commodity(&posting.commodity).to_string())
            .unwrap_or_default();
        // If the selected posting has no commodity, inherit it from any
        // other posting in the transaction that declares one.
        let commodity = if commodity.is_empty() {
            postings
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != *index)
                .find_map(|(_, posting)| {
                    let c = clean_commodity(&posting.commodity);
                    if c.is_empty() {
                        None
                    } else {
                        Some(c.to_string())
                    }
                })
                .unwrap_or_default()
        } else {
            commodity
        };
        if let Some((_, total)) = parts
            .iter_mut()
            .find(|(existing_commodity, _)| existing_commodity == &commodity)
        {
            *total += value.abs();
        } else {
            parts.push((commodity, value.abs()));
        }
    }

    parts.retain(|(_, total)| *total != 0.0);
    if parts.is_empty() {
        return None;
    }

    let amount = parts
        .iter()
        .map(|(commodity, total)| format_amount_part(*total, commodity, false))
        .collect::<Vec<_>>()
        .join(" + ");
    let formatted = if !formatted_with_commodity && parts.len() == 1 {
        format_amount_value_styled(parts[0].1)
    } else {
        parts
            .iter()
            .map(|(commodity, total)| format_amount_part(*total, commodity, true))
            .collect::<Vec<_>>()
            .join(" + ")
    };

    Some(DisplayAmount {
        amount: kind
            .map(|kind| format_display_amount(&amount, kind))
            .unwrap_or(amount),
        formatted,
    })
}

fn clean_commodity(commodity: &str) -> &str {
    commodity
        .split("@@")
        .next()
        .unwrap_or(commodity)
        .split('@')
        .next()
        .unwrap_or(commodity)
        .trim()
}

fn format_amount_part(value: f64, commodity: &str, styled: bool) -> String {
    let commodity = clean_commodity(commodity);
    if styled {
        if commodity.chars().all(|character| character.is_alphabetic()) {
            return format!("{} {}", format_commodity_quantity(value), commodity);
        }
        let style_ref = AMOUNT_STYLE.get().unwrap_or_else(|| {
            static DEFAULT_STYLE: OnceLock<AmountStyle> = OnceLock::new();
            DEFAULT_STYLE.get_or_init(AmountStyle::default)
        });
        return style_ref.format_amount(value, commodity);
    }

    if commodity.is_empty() {
        return format_amount_quantity(value);
    }

    if commodity.chars().all(|character| character.is_alphabetic()) {
        format!("{} {}", format_commodity_quantity(value), commodity)
    } else {
        format!("{}{}", commodity, format_amount_quantity(value))
    }
}

fn summarize_transaction_flow(
    postings: &[JournalPosting],
    values: &[Option<f64>],
) -> TransactionFlow {
    let mut from = Vec::new();
    let mut to = Vec::new();

    for (posting, value) in postings.iter().zip(values.iter()) {
        let account = posting.account.trim();
        if account.is_empty() {
            continue;
        }

        match value {
            Some(value) if *value < 0.0 => push_unique(&mut from, account),
            Some(value) if *value > 0.0 => push_unique(&mut to, account),
            _ => {}
        }
    }

    if from.is_empty() && to.is_empty() {
        let accounts = postings
            .iter()
            .map(|posting| posting.account.trim())
            .filter(|account| !account.is_empty())
            .collect::<Vec<_>>();
        if accounts.len() > 1 {
            push_unique(&mut from, accounts[accounts.len() - 1]);
            push_unique(&mut to, accounts[0]);
        } else if let Some(account) = accounts.first() {
            push_unique(&mut to, account);
        }
    }

    TransactionFlow { from, to }
}

fn push_unique(accounts: &mut Vec<String>, account: &str) {
    if !accounts.iter().any(|existing| existing == account) {
        accounts.push(account.to_string());
    }
}

/// Parses a numeric value from an amount quantity.
fn parse_amount_value(amount: &str) -> f64 {
    let compact = amount.trim().replace(char::is_whitespace, "");
    if compact.is_empty() {
        return 0.0;
    }

    let last_comma = compact.rfind(',');
    let last_dot = compact.rfind('.');
    let decimal_index = match (last_comma, last_dot) {
        (Some(comma), Some(dot)) => Some(comma.max(dot)),
        (Some(comma), None) => Some(comma),
        (None, Some(dot)) => {
            let style_decimal_mark = AMOUNT_STYLE
                .get()
                .map(|style| style.decimal_mark.as_str())
                .unwrap_or(".");
            let fraction_len = compact[dot + 1..]
                .chars()
                .filter(|character| character.is_ascii_digit())
                .count();
            if style_decimal_mark == "," && fraction_len == 3 {
                None
            } else {
                Some(dot)
            }
        }
        (None, None) => None,
    };

    let mut normalized = String::new();
    for (index, character) in compact.char_indices() {
        if character.is_ascii_digit() || (character == '-' && normalized.is_empty()) {
            normalized.push(character);
        } else if Some(index) == decimal_index {
            normalized.push('.');
        }
    }

    normalized.parse::<f64>().unwrap_or_default()
}

fn format_amount_quantity(value: f64) -> String {
    let rounded = format!("{:.2}", value);
    rounded
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn format_commodity_quantity(value: f64) -> String {
    let rounded = format!("{:.8}", value);
    rounded
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn format_amount_value_styled(value: f64) -> String {
    let style_ref = AMOUNT_STYLE.get().unwrap_or_else(|| {
        static DEFAULT_STYLE: OnceLock<AmountStyle> = OnceLock::new();
        DEFAULT_STYLE.get_or_init(AmountStyle::default)
    });
    style_ref.format(value)
}

/// Formats a numeric string using the journal's display style.
fn format_amount_styled_with_commodity(raw: &str) -> String {
    let (quantity, commodity) = parse_posting_amount(raw);
    let value = parse_amount_value(&quantity).abs();
    format_amount_part(value, &commodity, true)
}

/// Formats the display amount sign according to the inferred transaction kind.
fn format_display_amount(amount: &str, kind: &str) -> String {
    let normalized = amount.replace('-', "");
    match kind {
        "income" => format!("+{}", normalized),
        "expense" => format!("-{}", normalized),
        _ => amount.to_string(),
    }
}
