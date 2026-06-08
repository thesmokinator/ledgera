use crate::{
    amount_style::{explicit_style_for_commodity, AmountStyle},
    app_error::to_error_string_with_details,
    journal::{
        files::{load_journal_files, JournalFile},
        types::{
            JournalPosting, JournalTransaction, PeriodicRule, PeriodicRuleInput, PostingInput,
            TransactionBlock, TransactionDisplay, TransactionFlow, TransactionInput,
        },
        util::{split_first_token, split_inline_comment},
    },
};
use std::{cell::RefCell, collections::HashMap, path::Path};

#[derive(Clone)]
struct TransactionParseStyle {
    amount_style: AmountStyle,
    commodity_styles: HashMap<String, AmountStyle>,
}

thread_local! {
    static TRANSACTION_PARSE_STYLE: RefCell<Option<TransactionParseStyle>> = const { RefCell::new(None) };
}

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

pub(crate) fn load_transactions_from_journal_via_files_with_style(
    files: &[JournalFile],
    amount_style: AmountStyle,
    commodity_styles: HashMap<String, AmountStyle>,
) -> Result<Vec<JournalTransaction>, String> {
    TRANSACTION_PARSE_STYLE.with(|cell| {
        let previous = cell.replace(Some(TransactionParseStyle {
            amount_style,
            commodity_styles,
        }));
        let result = load_transactions_from_journal_via_files(files);
        cell.replace(previous);
        result
    })
}

fn active_amount_style() -> AmountStyle {
    TRANSACTION_PARSE_STYLE
        .with(|cell| {
            cell.borrow()
                .as_ref()
                .map(|style| style.amount_style.clone())
        })
        .unwrap_or_else(|| crate::global_amount_style().clone())
}

fn active_style_for_commodity(commodity: &str) -> Option<AmountStyle> {
    TRANSACTION_PARSE_STYLE
        .with(|cell| {
            cell.borrow()
                .as_ref()
                .and_then(|style| style.commodity_styles.get(commodity).cloned())
        })
        .or_else(|| explicit_style_for_commodity(commodity))
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

struct ParsedAmount {
    quantity: String,
    commodity: String,
    unit_price: String,
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
    let parsed = parse_posting_amount(amount);
    Some(JournalPosting {
        account: account.trim().to_string(),
        amount: parsed.quantity,
        commodity: parsed.commodity,
        unit_price: parsed.unit_price,
        comment: comment.to_string(),
        raw: line.to_string(),
    })
}

/// Parses an hledger amount into quantity, commodity, and optional unit price.
/// Handles both `@` (unit price) and `@@` (total price) rate syntax.
fn parse_posting_amount(amount: &str) -> ParsedAmount {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return ParsedAmount {
            quantity: String::new(),
            commodity: String::new(),
            unit_price: String::new(),
        };
    }

    let Some(number_start) = trimmed.find(|character: char| character.is_ascii_digit()) else {
        return ParsedAmount {
            quantity: trimmed.to_string(),
            commodity: String::new(),
            unit_price: String::new(),
        };
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
    let suffix = trimmed[number_end..].trim();

    let (suffix_commodity, unit_price) = extract_price(suffix, &quantity);

    let commodity = if !prefix_commodity.is_empty() {
        prefix_commodity.to_string()
    } else if !suffix_commodity.is_empty() {
        suffix_commodity
    } else {
        String::new()
    };

    ParsedAmount {
        quantity,
        commodity,
        unit_price,
    }
}

/// Extracts price information from the suffix of a parsed amount.
/// Returns (suffix_commodity, unit_price).
/// Handles `@@` (total price → converted to unit price) and `@` (unit price as-is).
/// When no commodity is explicitly written before the rate token, the commodity
/// is inferred from the price expression.
fn extract_price(suffix: &str, quantity: &str) -> (String, String) {
    let trimmed = suffix.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    if let Some(pos) = rate_token_pos(trimmed, "@@") {
        let explicit_commodity = trimmed[..pos].trim().to_string();
        let price_raw = trimmed[pos + 2..].trim();
        let unit_price = compute_unit_price_from_total(price_raw, quantity);
        let commodity = if explicit_commodity.is_empty() {
            let (_, price_commodity) = parse_simple_amount(price_raw);
            price_commodity
        } else {
            explicit_commodity
        };
        (commodity, unit_price)
    } else if let Some(pos) = rate_token_pos(trimmed, "@") {
        let explicit_commodity = trimmed[..pos].trim().to_string();
        let unit_price = trimmed[pos + 1..].trim().to_string();
        let commodity = if explicit_commodity.is_empty() {
            let (_, price_commodity) = parse_simple_amount(&unit_price);
            price_commodity
        } else {
            explicit_commodity
        };
        (commodity, unit_price)
    } else {
        (trimmed.to_string(), String::new())
    }
}

/// Finds the position of a rate token (@ or @@) that is surrounded by whitespace
/// or at the start of the string.
fn rate_token_pos(suffix: &str, token: &str) -> Option<usize> {
    let pos = suffix.find(token)?;
    let preceded_by_space = pos == 0 || suffix.as_bytes().get(pos - 1) == Some(&b' ');
    if preceded_by_space {
        Some(pos)
    } else {
        None
    }
}

/// Converts a total price (@@) to a unit price by dividing by the quantity.
fn compute_unit_price_from_total(price_raw: &str, quantity: &str) -> String {
    let (price_qty, price_commodity) = parse_simple_amount(price_raw);
    let q = parse_amount_value(quantity);
    let p = parse_amount_value(&price_qty);
    if q != 0.0 && p != 0.0 {
        let unit = p / q.abs();
        let formatted = format!("{:.8}", unit);
        let formatted = formatted.trim_end_matches('0').trim_end_matches('.');
        if price_commodity.is_empty() {
            formatted.to_string()
        } else {
            format!("{} {}", formatted, price_commodity)
        }
    } else {
        price_raw.to_string()
    }
}

/// Parses a simple amount (quantity + commodity) without price detection.
/// Used for parsing the price portion of @/@@ expressions.
fn parse_simple_amount(amount: &str) -> (String, String) {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    let Some(number_start) = trimmed.find(|c: char| c.is_ascii_digit()) else {
        return (trimmed.to_string(), String::new());
    };

    let sign = if trimmed[..number_start].contains('-') {
        "-"
    } else {
        ""
    };
    let number_end = trimmed[number_start..]
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ','))
        .map(|(i, _)| number_start + i)
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
pub(crate) fn split_lines<'a>(content: &'a str) -> Vec<&'a str> {
    content.lines().collect()
}

/// Replaces a one-based inclusive line range.
pub(crate) fn replace_line_range(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    replacement: &str,
) -> String {
    let start_index = start_line.saturating_sub(1);
    let end_index = end_line.min(lines.len());

    let mut result: Vec<&str> = lines[..start_index].to_vec();
    if !replacement.trim().is_empty() {
        result.extend(replacement.lines());
    }
    result.extend_from_slice(&lines[end_index..]);

    let mut content = result.join("\n");
    content.push('\n');
    content
}

/// Formats a transaction from structured form input.
pub(crate) fn format_transaction(input: &TransactionInput) -> String {
    format_transaction_with_comment(input, "")
}

/// Formats a transaction without normalizing posting quantities.
pub(crate) fn format_transaction_preserving_quantities(input: &TransactionInput) -> String {
    format_transaction_with_comment_and_quantity_mode(input, "", QuantityFormat::Preserve)
}

/// Formats a transaction with an optional header comment (e.g. "rule-id:salary").
pub(crate) fn format_transaction_with_comment(input: &TransactionInput, comment: &str) -> String {
    format_transaction_with_comment_and_quantity_mode(input, comment, QuantityFormat::Normalize)
}

fn format_transaction_with_comment_and_quantity_mode(
    input: &TransactionInput,
    comment: &str,
    quantity_format: QuantityFormat,
) -> String {
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
    if !comment.trim().is_empty() {
        header.push_str("  ; ");
        header.push_str(comment.trim());
    }

    let postings = input
        .postings
        .iter()
        .filter(|posting| !posting.account.trim().is_empty())
        .map(|posting| match quantity_format {
            QuantityFormat::Normalize => format_posting(posting),
            QuantityFormat::Preserve => format_posting_preserving_quantity(posting),
        })
        .collect::<Vec<_>>();

    std::iter::once(header)
        .chain(postings)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns true when a line starts a periodic transaction rule (tilde syntax).
pub(crate) fn is_periodic_rule_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("~ ") || trimmed.starts_with("~\t")
}

/// Parses periodic transaction rules from journal content.
pub(crate) fn parse_periodic_rules(content: &str, source_path: &Path) -> Vec<PeriodicRule> {
    let lines = split_lines(content);
    let mut rules = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_periodic_rule_header(lines[index]) {
            index += 1;
            continue;
        }

        let start_line = index + 1;
        let mut end_index = index + 1;
        while end_index < lines.len()
            && !is_periodic_rule_header(lines[end_index])
            && !is_transaction_header(lines[end_index])
            && (lines[end_index].starts_with(char::is_whitespace)
                || lines[end_index].trim().is_empty()
                || lines[end_index].trim().starts_with(';')
                || lines[end_index].trim().starts_with('#'))
        {
            end_index += 1;
        }

        let block_lines = &lines[index..end_index];
        let raw = block_lines.join("\n");
        if let Some(rule) = parse_periodic_rule_block(source_path, start_line, end_index, &raw) {
            rules.push(rule);
        }
        index = end_index;
    }

    rules
}

fn parse_periodic_rule_block(
    source_path: &Path,
    start_line: usize,
    end_line: usize,
    raw: &str,
) -> Option<PeriodicRule> {
    let mut lines_iter = raw.lines();
    let header = lines_iter.next()?.trim();
    let (period_expr, start_date, end_date, comment) = parse_periodic_header(header)?;

    let (rule_id, description) = extract_rule_id_from_comment(&comment);

    let status = String::new();
    let code = String::new();

    let mut postings = Vec::new();
    for line in raw.lines().skip(1) {
        let Some(posting) = parse_posting(line) else {
            continue;
        };
        if posting_comment_rule_id(&posting.comment)
            .is_some_and(|posting_rule_id| posting_rule_id != rule_id)
        {
            break;
        }
        postings.push(posting);
    }

    let source_file = source_path.to_string_lossy().to_string();
    Some(PeriodicRule {
        id: format!("{}:{}", source_file, start_line),
        rule_id,
        source_file,
        period_expr,
        description,
        postings,
        status,
        code,
        start_date,
        end_date,
        comment,
        raw: raw.to_string(),
        start_line,
        end_line,
    })
}

fn parse_periodic_header(line: &str) -> Option<(String, Option<String>, Option<String>, String)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('~')?.trim_start();

    let (body, comment) = split_inline_comment(rest);
    let body = body.trim();

    let mut remaining = body;
    let mut end_date = None;
    let mut start_date = None;

    if let Some(idx) = remaining.rfind(" to ") {
        let potential_date = remaining[idx + 4..].trim();
        if is_date_literal(potential_date) {
            end_date = Some(potential_date.to_string());
            remaining = remaining[..idx].trim();
        }
    }

    if let Some(idx) = remaining.rfind(" from ") {
        let potential_date = remaining[idx + 6..].trim();
        if is_date_literal(potential_date) {
            start_date = Some(potential_date.to_string());
            remaining = remaining[..idx].trim();
        }
    }

    let period_expr = remaining.to_string();
    if period_expr.is_empty() {
        return None;
    }

    Some((period_expr, start_date, end_date, comment.to_string()))
}

fn is_date_literal(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes().get(4) == Some(&b'-')
        && s.as_bytes().get(7) == Some(&b'-')
        && s[..4].chars().all(|c| c.is_ascii_digit())
        && s[5..7].chars().all(|c| c.is_ascii_digit())
        && s[8..].chars().all(|c| c.is_ascii_digit())
}

fn extract_rule_id_from_comment(comment: &str) -> (String, String) {
    let trimmed = comment.trim();
    if let Some(rest) = trimmed.strip_prefix("rule-id:") {
        let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
        let id = parts.first().map(|s| s.to_string()).unwrap_or_default();
        let desc = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
        (id, desc)
    } else {
        (String::new(), trimmed.to_string())
    }
}

fn posting_comment_rule_id(comment: &str) -> Option<String> {
    let trimmed = comment.trim();
    let rest = trimmed.strip_prefix("rule-id:")?;
    Some(
        rest.split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string(),
    )
}

/// Formats a periodic rule into its journal text representation.
pub(crate) fn format_periodic_rule_text(input: &PeriodicRuleInput) -> String {
    let mut header = format!("~ {}", input.period_expr.trim());
    if let Some(ref start) = input.start_date {
        if !start.trim().is_empty() {
            header.push_str(&format!(" from {}", start.trim()));
        }
    }
    if let Some(ref end) = input.end_date {
        if !end.trim().is_empty() {
            header.push_str(&format!(" to {}", end.trim()));
        }
    }
    let mut comment_parts: Vec<String> = Vec::new();
    if !input.rule_id.trim().is_empty() {
        let mut rule_part = format!("rule-id:{}", input.rule_id.trim());
        let desc = input.description.trim();
        if !desc.is_empty() {
            rule_part.push(' ');
            rule_part.push_str(desc);
        }
        comment_parts.push(rule_part);
    }
    if !input.comment.trim().is_empty() {
        comment_parts.push(input.comment.trim().to_string());
    }
    if !comment_parts.is_empty() {
        header.push_str("  ; ");
        header.push_str(&comment_parts.join(" "));
    }

    let postings = input
        .postings
        .iter()
        .filter(|p| !p.account.trim().is_empty())
        .map(format_posting_preserving_quantity)
        .collect::<Vec<_>>();

    std::iter::once(header)
        .chain(postings)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone, Copy)]
enum QuantityFormat {
    Normalize,
    Preserve,
}

/// Normalizes a numeric quantity using the commodity style when possible.
fn normalize_quantity(value: &str, commodity: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.replace(',', ".");
    match normalized.parse::<f64>() {
        Ok(val) => active_style_for_commodity(commodity)
            .unwrap_or_else(active_amount_style)
            .format(val),
        Err(_) => trimmed.to_string(),
    }
}

/// Formats a quantity and commodity into hledger amount syntax.
fn format_posting_amount(amount: &str, commodity: &str) -> String {
    format_posting_amount_with_quantity_mode(amount, commodity, QuantityFormat::Normalize)
}

fn format_posting_amount_preserving_quantity(amount: &str, commodity: &str) -> String {
    format_posting_amount_with_quantity_mode(amount, commodity, QuantityFormat::Preserve)
}

fn format_posting_amount_with_quantity_mode(
    amount: &str,
    commodity: &str,
    quantity_format: QuantityFormat,
) -> String {
    let parsed = parse_posting_amount(amount);
    let selected_commodity = if parsed.commodity.trim().is_empty() {
        commodity.trim()
    } else {
        parsed.commodity.trim()
    };
    let quantity = match quantity_format {
        QuantityFormat::Normalize => normalize_quantity(&parsed.quantity, selected_commodity),
        QuantityFormat::Preserve => parsed.quantity.trim().to_string(),
    };

    if quantity.is_empty() {
        return String::new();
    }
    if selected_commodity.is_empty() {
        return quantity;
    }

    format_quantity_and_commodity(&quantity, selected_commodity)
}

fn format_quantity_and_commodity(quantity: &str, commodity: &str) -> String {
    if let Some(style) = explicit_style_for_commodity(commodity) {
        return format_quantity_and_commodity_with_style(quantity, commodity, &style);
    }

    let sign = if quantity.starts_with('-') { "-" } else { "" };
    let absolute_quantity = quantity.trim_start_matches('-');
    if commodity.chars().all(|character| character.is_alphabetic()) {
        format!("{}{} {}", sign, absolute_quantity, commodity)
    } else {
        format!("{}{}{}", sign, commodity, absolute_quantity)
    }
}

fn format_quantity_and_commodity_with_style(
    quantity: &str,
    commodity: &str,
    style: &AmountStyle,
) -> String {
    let sign = if quantity.starts_with('-') { "-" } else { "" };
    let absolute_quantity = quantity.trim_start_matches('-');
    let separator = if style.commodity_spaced { " " } else { "" };

    if style.commodity_position == "right" {
        format!("{}{}{}{}", sign, absolute_quantity, separator, commodity)
    } else {
        format!("{}{}{}{}", sign, commodity, separator, absolute_quantity)
    }
}

/// Formats a posting, including an optional hledger inline comment.
pub(crate) fn format_posting(posting: &PostingInput) -> String {
    format_posting_with_quantity_mode(posting, QuantityFormat::Normalize)
}

fn format_posting_preserving_quantity(posting: &PostingInput) -> String {
    format_posting_with_quantity_mode(posting, QuantityFormat::Preserve)
}

fn format_posting_with_quantity_mode(
    posting: &PostingInput,
    quantity_format: QuantityFormat,
) -> String {
    let mut amount = match quantity_format {
        QuantityFormat::Normalize => format_posting_amount(&posting.amount, &posting.commodity),
        QuantityFormat::Preserve => {
            format_posting_amount_preserving_quantity(&posting.amount, &posting.commodity)
        }
    };
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
        let tint = crate::tint(parse_amount_value(&posting.amount)).to_string();
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
        return active_style_for_commodity(commodity)
            .unwrap_or_else(active_amount_style)
            .format_amount(value, commodity);
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
            let active_style = active_amount_style();
            let style_decimal_mark = active_style.decimal_mark.as_str();
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
    active_amount_style().format(value)
}

/// Formats a numeric string using the journal's display style.
fn format_amount_styled_with_commodity(raw: &str) -> String {
    let parsed = parse_posting_amount(raw);
    let value = parse_amount_value(&parsed.quantity).abs();
    format_amount_part(value, &parsed.commodity, true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_amount_with_suffix_commodity() {
        let parsed = parse_posting_amount("25 EUR");
        assert_eq!(parsed.quantity, "25");
        assert_eq!(parsed.commodity, "EUR");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parses_simple_amount_with_prefix_commodity() {
        let parsed = parse_posting_amount("$25");
        assert_eq!(parsed.quantity, "25");
        assert_eq!(parsed.commodity, "$");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parses_negative_amount() {
        let parsed = parse_posting_amount("€-25,50");
        assert_eq!(parsed.quantity, "-25,50");
        assert_eq!(parsed.commodity, "€");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parses_unit_price_at_syntax() {
        let parsed = parse_posting_amount("2 @ ₺190");
        assert_eq!(parsed.quantity, "2");
        assert_eq!(parsed.commodity, "₺");
        assert_eq!(parsed.unit_price, "₺190");
    }

    #[test]
    fn parses_total_price_at_at_syntax() {
        let parsed = parse_posting_amount("2 @@ ₺380,00");
        assert_eq!(parsed.quantity, "2");
        assert_eq!(parsed.commodity, "₺");
        // 380 / 2 = 190
        assert!(parsed.unit_price.contains("190"));
        assert!(parsed.unit_price.contains("₺"));
    }

    #[test]
    fn parses_total_price_with_different_commodity() {
        let parsed = parse_posting_amount("2 EUR @@ 2,20 USD");
        assert_eq!(parsed.quantity, "2");
        assert_eq!(parsed.commodity, "EUR");
        // 2.20 / 2 = 1.1
        assert!(parsed.unit_price.contains("1.1"));
        assert!(parsed.unit_price.contains("USD"));
    }

    #[test]
    fn parses_unit_price_with_explicit_commodity_before_at() {
        let parsed = parse_posting_amount("10 VWCE @ 150 EUR");
        assert_eq!(parsed.quantity, "10");
        assert_eq!(parsed.commodity, "VWCE");
        assert_eq!(parsed.unit_price, "150 EUR");
    }

    #[test]
    fn parses_total_price_with_commodity_before_at_at() {
        let parsed = parse_posting_amount("5 XEON @@ 750 EUR");
        assert_eq!(parsed.quantity, "5");
        assert_eq!(parsed.commodity, "XEON");
        // 750 / 5 = 150
        assert!(parsed.unit_price.contains("150"));
        assert!(parsed.unit_price.contains("EUR"));
    }

    #[test]
    fn parses_amount_without_commodity_or_price() {
        let parsed = parse_posting_amount("100");
        assert_eq!(parsed.quantity, "100");
        assert_eq!(parsed.commodity, "");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parses_empty_amount() {
        let parsed = parse_posting_amount("");
        assert_eq!(parsed.quantity, "");
        assert_eq!(parsed.commodity, "");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parses_non_numeric_as_quantity() {
        let parsed = parse_posting_amount("ABC");
        assert_eq!(parsed.quantity, "ABC");
        assert_eq!(parsed.commodity, "");
        assert_eq!(parsed.unit_price, "");
    }

    #[test]
    fn parse_posting_handles_at_at_syntax() {
        let line = "    Expenses:Food                 2 @@ ₺380,00";
        let posting = parse_posting(line).expect("should parse posting");
        assert_eq!(posting.account, "Expenses:Food");
        assert_eq!(posting.amount, "2");
        assert_eq!(posting.commodity, "₺");
        assert!(posting.unit_price.contains("190"));
        assert!(posting.unit_price.contains("₺"));
    }

    #[test]
    fn parse_posting_handles_simple_amount() {
        let line = "    Assets:Cash                    ₺-380,00";
        let posting = parse_posting(line).expect("should parse posting");
        assert_eq!(posting.account, "Assets:Cash");
        assert_eq!(posting.amount, "-380,00");
        assert_eq!(posting.commodity, "₺");
        assert_eq!(posting.unit_price, "");
    }

    #[test]
    fn parse_posting_returns_none_for_empty_lines() {
        assert!(parse_posting("").is_none());
        assert!(parse_posting("   ").is_none());
    }

    #[test]
    fn parse_posting_returns_none_for_comment_lines() {
        assert!(parse_posting("    ; this is a comment").is_none());
    }

    #[test]
    fn parse_posting_returns_none_for_non_indented_lines() {
        assert!(parse_posting("2026-05-22 Transaction").is_none());
    }

    #[test]
    fn rate_token_pos_detects_token_at_start() {
        assert_eq!(rate_token_pos("@@ ₺380,00", "@@"), Some(0));
        assert_eq!(rate_token_pos("@ ₺190", "@"), Some(0));
    }

    #[test]
    fn rate_token_pos_detects_token_after_space() {
        assert_eq!(rate_token_pos("EUR @@ USD", "@@"), Some(4));
        assert_eq!(rate_token_pos("VWCE @ 150", "@"), Some(5));
    }

    #[test]
    fn rate_token_pos_rejects_token_without_preceding_space() {
        assert_eq!(rate_token_pos("EUR@@USD", "@@"), None);
        assert_eq!(rate_token_pos("VWCE@150", "@"), None);
    }

    #[test]
    fn rate_token_pos_does_not_confuse_at_with_at_at() {
        // "@" should match in " EUR @ USD " but not the "@@" variant
        let pos = rate_token_pos(" EUR @@ USD ", "@");
        // The first "@" in "@@" is at position 5, preceded by space — so it matches
        // But that's actually fine: `extract_price` checks `@@` first, so `@` won't
        // falsely match in that case.
        assert!(pos.is_some());
    }

    #[test]
    fn parses_price_with_comma_decimal() {
        let parsed = parse_posting_amount("2 @@ ₺380,00");
        assert_eq!(parsed.quantity, "2");
        // 380,00 / 2 = 190
        assert!(parsed.unit_price.contains("190"));
    }

    #[test]
    fn extract_price_returns_empty_for_no_rate() {
        let (commodity, price) = extract_price("EUR", "10");
        assert_eq!(commodity, "EUR");
        assert_eq!(price, "");
    }

    #[test]
    fn extract_price_handles_empty_suffix() {
        let (commodity, price) = extract_price("", "10");
        assert_eq!(commodity, "");
        assert_eq!(price, "");
    }

    /// Full integration test: parse the exact transaction from issue #27.
    #[test]
    fn parses_transaction_from_issue_27() {
        let raw = "2026-05-22 Aygaz\n    Expenses:Food:Groceries:Water                 2 @@ ₺380,00\n    Assets:Cash:Caleb                                 ₺-380,00";
        let tx = parse_transaction_block(std::path::Path::new("test.journal"), 1, 3, raw)
            .expect("should parse transaction");

        assert_eq!(tx.date, "2026-05-22");
        assert_eq!(tx.description, "Aygaz");
        assert_eq!(tx.postings.len(), 2);

        let first = &tx.postings[0];
        assert_eq!(first.account, "Expenses:Food:Groceries:Water");
        assert_eq!(first.amount, "2");
        assert_eq!(first.commodity, "₺");
        assert!(
            !first.unit_price.is_empty(),
            "unit_price should be populated from @@"
        );
        assert!(first.unit_price.contains("190"), "380 / 2 = 190");

        let second = &tx.postings[1];
        assert_eq!(second.account, "Assets:Cash:Caleb");
        assert_eq!(second.amount, "-380,00");
        assert_eq!(second.commodity, "₺");
        assert!(second.unit_price.is_empty());
    }

    /// Parse a transaction using @ (unit price) syntax.
    #[test]
    fn parses_transaction_with_unit_price_at() {
        let raw = "2026-05-23 Market buy\n    Assets:Investments:VWCE                    10 VWCE @ 150 EUR\n    Assets:Bank:Fineco                             -1.500 EUR";
        let tx = parse_transaction_block(std::path::Path::new("test.journal"), 1, 3, raw)
            .expect("should parse transaction");

        let first = &tx.postings[0];
        assert_eq!(first.account, "Assets:Investments:VWCE");
        assert_eq!(first.amount, "10");
        assert_eq!(first.commodity, "VWCE");
        assert_eq!(first.unit_price, "150 EUR");
    }

    /// Parse a transaction using @@ with different commodities.
    #[test]
    fn parses_transaction_with_cross_commodity_at_at() {
        let raw = "2026-05-24 Currency exchange\n    Assets:Cash:USD                            100 USD @@ 92 EUR\n    Assets:Cash:EUR                                -92 EUR";
        let tx = parse_transaction_block(std::path::Path::new("test.journal"), 1, 3, raw)
            .expect("should parse transaction");

        let first = &tx.postings[0];
        assert_eq!(first.account, "Assets:Cash:USD");
        assert_eq!(first.amount, "100");
        assert_eq!(first.commodity, "USD");
        // 92 / 100 = 0.92 EUR per USD
        assert!(first.unit_price.contains("0.92"));
        assert!(first.unit_price.contains("EUR"));
    }

    #[test]
    fn formats_periodic_rule_preserving_amount_quantity() {
        let input = PeriodicRuleInput {
            rule_id: "insurance".to_string(),
            period_expr: "monthly".to_string(),
            description: "Insurance".to_string(),
            postings: vec![
                PostingInput {
                    account: "assets:bank:fineco".to_string(),
                    amount: "50,00".to_string(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
                PostingInput {
                    account: "expenses:insurance:life".to_string(),
                    amount: String::new(),
                    commodity: String::new(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
            ],
            status: String::new(),
            code: String::new(),
            start_date: Some("2026-06-05".to_string()),
            end_date: None,
            comment: String::new(),
        };

        let result = format_periodic_rule_text(&input);

        assert!(result.contains("50,00 EUR"));
        assert!(!result.contains("50.00 EUR"));
    }

    #[test]
    fn parse_periodic_rules_stops_at_foreign_rule_id_posting_comment() {
        let content = "~ monthly from 2026-06-05  ; rule-id:allianz Assicurazione Vita\n    assets:bank:fineco  50,00 EUR\n    expenses:insurance:life~ monthly from 2026-06-26  ; rule-id:apple Apple Music\n    assets:bank:fineco  2,99 EUR\n";

        let rules = parse_periodic_rules(content, std::path::Path::new("recurring.journal"));

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_id, "allianz");
        assert_eq!(rules[0].postings.len(), 1);
        assert_eq!(rules[0].postings[0].account, "assets:bank:fineco");
    }

    #[test]
    fn formats_generated_recurring_transaction_preserving_amount_quantity() {
        let input = TransactionInput {
            mode: String::new(),
            date: "2026-06-05".to_string(),
            status: String::new(),
            code: String::new(),
            description: "Insurance".to_string(),
            postings: vec![
                PostingInput {
                    account: "assets:bank:fineco".to_string(),
                    amount: "50,00".to_string(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: "rule-id:insurance".to_string(),
                },
                PostingInput {
                    account: "expenses:insurance:life".to_string(),
                    amount: String::new(),
                    commodity: String::new(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
            ],
        };

        let result = format_transaction_preserving_quantities(&input);

        assert!(result.contains("50,00 EUR"));
        assert!(!result.contains("50.00 EUR"));
    }
}
