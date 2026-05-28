use crate::{
    app_error::{to_error_string_with_details, to_validation_error_string, FieldError},
    hledger::hledger_executable,
    journal::{
        files::{parse_include_directive, require_journal_path},
        parser::{find_block, format_transaction, replace_line_range, split_lines},
        summary::read_journal_summary,
        types::{JournalSummary, RoutingStrategy, TransactionInput},
    },
    settings::AppSettings,
};
use chrono::NaiveDate;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn create_transaction_for_settings(
    settings: &AppSettings,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    validate_transaction_input(input)?;
    let journal_path = require_journal_path(settings)?;
    append_transaction_routed(settings, &journal_path, input)?;
    read_journal_summary(&journal_path, settings.default_commodity.trim())
}

pub(crate) fn update_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
    input: &TransactionInput,
) -> Result<JournalSummary, String> {
    validate_transaction_input(input)?;
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    let replacement = format_transaction(input);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            &replacement,
        ))
    })?;
    read_journal_summary(&journal_path, settings.default_commodity.trim())
}

pub(crate) fn delete_transaction_for_settings(
    settings: &AppSettings,
    id: &str,
) -> Result<JournalSummary, String> {
    let journal_path = require_journal_path(settings)?;
    let block = find_block(&journal_path, id)?;
    let source_path = PathBuf::from(&block.transaction.source_file);
    mutate_existing_file(settings, &journal_path, &source_path, |content| {
        let lines = split_lines(content);
        Ok(replace_line_range(
            &lines,
            block.transaction.start_line,
            block.transaction.end_line,
            "",
        ))
    })?;
    read_journal_summary(&journal_path, settings.default_commodity.trim())
}

fn validate_transaction_input(input: &TransactionInput) -> Result<(), String> {
    let mut errors = Vec::new();
    let mode = input.mode.trim();

    validate_single_line_field(
        &mut errors,
        &["mode"],
        &input.mode,
        "Mode cannot contain line breaks.",
    );
    if !mode.is_empty() && !matches!(mode, "movement" | "investment" | "advanced") {
        errors.push(FieldError::new(
            ["mode"],
            "Mode must be movement, investment, or advanced.",
        ));
    }

    validate_single_line_field(
        &mut errors,
        &["date"],
        &input.date,
        "Date cannot contain line breaks.",
    );
    if input.date.trim().is_empty() {
        errors.push(FieldError::new(["date"], "Date is required."));
    } else if NaiveDate::parse_from_str(input.date.trim(), "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            ["date"],
            "Use a valid date in YYYY-MM-DD format.",
        ));
    }

    validate_single_line_field(
        &mut errors,
        &["status"],
        &input.status,
        "Status cannot contain line breaks.",
    );
    let status = input.status.trim();
    if !status.is_empty() && status != "*" && status != "!" {
        errors.push(FieldError::new(["status"], "Status must be empty, * or !."));
    }

    validate_single_line_field(
        &mut errors,
        &["code"],
        &input.code,
        "Code cannot contain line breaks.",
    );
    let code = input.code.trim();
    let has_valid_hledger_code = code.starts_with('(') && code.ends_with(')') && code.len() > 2;
    if !(code.is_empty() || has_valid_hledger_code) {
        errors.push(FieldError::new(
            ["code"],
            "Code must use hledger parentheses, for example (INV-001).",
        ));
    }

    validate_single_line_field(
        &mut errors,
        &["description"],
        &input.description,
        "Description cannot contain line breaks.",
    );

    let mut posting_accounts = 0usize;
    let mut postings_with_amounts = 0usize;

    for (index, posting) in input.postings.iter().enumerate() {
        let index = index.to_string();
        let account = posting.account.trim();
        let amount = posting.amount.trim();
        let commodity = posting.commodity.trim();
        let unit_price = posting.unit_price.trim();
        let comment = posting.comment.trim();
        let has_meaningful_input = !account.is_empty()
            || !amount.is_empty()
            || !unit_price.is_empty()
            || !comment.is_empty();

        if !has_meaningful_input {
            continue;
        }

        validate_single_line_field(
            &mut errors,
            &["postings", index.as_str(), "account"],
            &posting.account,
            "Account cannot contain line breaks.",
        );
        validate_single_line_field(
            &mut errors,
            &["postings", index.as_str(), "amount"],
            &posting.amount,
            "Amount cannot contain line breaks.",
        );
        validate_single_line_field(
            &mut errors,
            &["postings", index.as_str(), "commodity"],
            &posting.commodity,
            "Commodity cannot contain line breaks.",
        );
        validate_single_line_field(
            &mut errors,
            &["postings", index.as_str(), "unitPrice"],
            &posting.unit_price,
            "Unit price cannot contain line breaks.",
        );
        validate_single_line_field(
            &mut errors,
            &["postings", index.as_str(), "comment"],
            &posting.comment,
            "Comment cannot contain line breaks.",
        );

        if account.is_empty() {
            errors.push(FieldError::new(
                ["postings", index.as_str(), "account"],
                "Account is required.",
            ));
            continue;
        }
        posting_accounts += 1;

        if !amount.is_empty() {
            postings_with_amounts += 1;
            if !amount_contains_number(amount) {
                errors.push(FieldError::new(
                    ["postings", index.as_str(), "amount"],
                    "Amount must contain a number.",
                ));
            }
            if commodity.is_empty() && !amount_contains_commodity(amount) {
                errors.push(FieldError::new(
                    ["postings", index.as_str(), "commodity"],
                    "Commodity is required when the amount does not include one.",
                ));
            }
        }

        if !unit_price.is_empty() {
            if amount.is_empty() {
                errors.push(FieldError::new(
                    ["postings", index.as_str(), "unitPrice"],
                    "Unit price requires a quantity.",
                ));
            }
            if !amount_contains_number(unit_price) {
                errors.push(FieldError::new(
                    ["postings", index.as_str(), "unitPrice"],
                    "Unit price must contain a number.",
                ));
            }
        }
    }

    if posting_accounts < 2 {
        errors.push(FieldError::new(
            ["postings"],
            "Add at least two posting accounts.",
        ));
    }
    if postings_with_amounts == 0 {
        errors.push(FieldError::new(
            ["postings"],
            "Enter an amount on at least one posting.",
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(to_validation_error_string(
            "transaction_validation_failed",
            "Transaction validation failed. Check the highlighted fields.",
            errors,
        ))
    }
}

fn validate_single_line_field(
    errors: &mut Vec<FieldError>,
    path: &[&str],
    value: &str,
    message: &str,
) {
    if value.contains('\n') || value.contains('\r') {
        errors.push(FieldError::new(path.iter().copied(), message));
    }
}

fn amount_contains_number(value: &str) -> bool {
    value.chars().any(|character| character.is_ascii_digit())
}

fn amount_contains_commodity(value: &str) -> bool {
    value.chars().any(|character| {
        !(character.is_ascii_digit()
            || character.is_whitespace()
            || matches!(character, '.' | ',' | '-' | '+'))
    })
}

fn mutate_existing_file<F>(
    settings: &AppSettings,
    main_journal: &Path,
    source_path: &Path,
    mutate: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let original = fs::read_to_string(source_path).map_err(|error| {
        to_error_string_with_details(
            "journal_read_failed",
            "Unable to read source journal file.",
            format!("{}: {}", source_path.display(), error),
        )
    })?;
    let updated = mutate(&original)?;

    fs::write(source_path, &updated).map_err(|error| {
        to_error_string_with_details(
            "journal_write_failed",
            "Unable to write journal file.",
            error.to_string(),
        )
    })?;
    if let Err(error) = validate_journal(settings, main_journal) {
        fs::write(source_path, original).map_err(|rollback_error| {
            to_error_string_with_details(
                "journal_write_failed",
                "Journal file may be corrupted - rollback failed.",
                rollback_error.to_string(),
            )
        })?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_routed(
    settings: &AppSettings,
    main_journal: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    let content = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    match detect_routing_strategy(&content) {
        RoutingStrategy::Fallback => {
            append_to_existing_file(settings, main_journal, main_journal, input)
        }
        RoutingStrategy::Flat(includes) => {
            let target_include = flat_target_include(&includes, input);
            let target_file = resolve_relative_to_main(main_journal, &target_include);
            if target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else {
                append_to_new_flat_subjournal(
                    settings,
                    main_journal,
                    &target_file,
                    &target_include,
                    input,
                )
            }
        }
        RoutingStrategy::Glob(includes) => {
            let (target_file, target_include) = glob_target_path(main_journal, &includes, input);
            if target_file.exists() {
                append_to_existing_file(settings, main_journal, &target_file, input)
            } else if glob_include_matches_year(&target_include, &includes) {
                append_to_new_glob_subjournal(settings, main_journal, &target_file, input)
            } else {
                append_to_new_glob_year(
                    settings,
                    main_journal,
                    &target_file,
                    &target_include,
                    input,
                )
            }
        }
    }
}

fn append_to_existing_file(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    mutate_existing_file(settings, main_journal, target_file, |content| {
        Ok(append_transaction_text(content, input))
    })
}

fn append_to_new_flat_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    target_name: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let updated_main = insert_include_sorted(&original_main, target_name);

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_subjournal(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    input: &TransactionInput,
) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        return Err(error);
    }

    Ok(())
}

fn append_to_new_glob_year(
    settings: &AppSettings,
    main_journal: &Path,
    target_file: &Path,
    target_include: &str,
    input: &TransactionInput,
) -> Result<(), String> {
    let original_main = fs::read_to_string(main_journal).map_err(|error| error.to_string())?;
    let year_dir = target_file
        .parent()
        .ok_or_else(|| "Unable to resolve target journal directory.".to_string())?;
    let year_dir_created = !year_dir.exists();
    let updated_main = insert_glob_include_sorted(&original_main, target_include);

    fs::write(main_journal, updated_main).map_err(|error| error.to_string())?;
    fs::create_dir_all(year_dir).map_err(|error| error.to_string())?;
    fs::write(target_file, format!("{}\n", format_transaction(input)))
        .map_err(|error| error.to_string())?;

    if let Err(error) = validate_journal(settings, main_journal) {
        let _ = fs::remove_file(target_file);
        if year_dir_created {
            let _ = fs::remove_dir(year_dir);
        }
        fs::write(main_journal, original_main)
            .map_err(|rollback_error| rollback_error.to_string())?;
        return Err(error);
    }

    Ok(())
}

fn append_transaction_text(content: &str, input: &TransactionInput) -> String {
    let mut updated = content.trim_end_matches(['\r', '\n']).to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&format_transaction(input));
    updated.push('\n');
    updated
}

fn detect_routing_strategy(content: &str) -> RoutingStrategy {
    let glob_includes = content
        .lines()
        .filter_map(parse_glob_include)
        .collect::<Vec<_>>();
    if !glob_includes.is_empty() {
        return RoutingStrategy::Glob(glob_includes);
    }

    let flat_files = content
        .lines()
        .filter_map(parse_flat_include_filename)
        .collect::<Vec<_>>();
    if !flat_files.is_empty() {
        return RoutingStrategy::Flat(flat_files);
    }

    RoutingStrategy::Fallback
}

fn parse_flat_include_filename(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    let file_name = Path::new(&include).file_name()?.to_str()?;
    if file_name.len() == "YYYY-MM.journal".len()
        && file_name.ends_with(".journal")
        && file_name
            .chars()
            .take(4)
            .all(|character| character.is_ascii_digit())
        && file_name.chars().nth(4) == Some('-')
        && file_name
            .chars()
            .skip(5)
            .take(2)
            .all(|character| character.is_ascii_digit())
    {
        Some(include)
    } else {
        None
    }
}

fn parse_glob_include(line: &str) -> Option<String> {
    let include = parse_include_directive(line)?;
    if include.ends_with("/*.journal") && glob_include_year(&include).is_some() {
        Some(include)
    } else {
        None
    }
}

fn glob_include_year(include: &str) -> Option<String> {
    let mut parts = include.split('/').collect::<Vec<_>>();
    if parts.pop()? != "*.journal" {
        return None;
    }
    parts
        .into_iter()
        .rev()
        .find(|part| part.len() == 4 && part.chars().all(|character| character.is_ascii_digit()))
        .map(ToString::to_string)
}

fn target_subjournal_name(input: &TransactionInput) -> String {
    format!("{}.journal", input.date.chars().take(7).collect::<String>())
}

fn flat_target_include(includes: &[String], input: &TransactionInput) -> String {
    let target_name = target_subjournal_name(input);
    if let Some(existing) = includes.iter().find(|include| {
        Path::new(include)
            .file_name()
            .and_then(|name| name.to_str())
            == Some(target_name.as_str())
    }) {
        return existing.clone();
    }

    let target_year = input.date.chars().take(4).collect::<String>();
    if let Some(similar_year_include) = includes.iter().find(|include| {
        Path::new(include)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with(&target_year))
            .unwrap_or(false)
    }) {
        return Path::new(similar_year_include)
            .parent()
            .map(|parent| parent.join(&target_name).to_string_lossy().to_string())
            .unwrap_or(target_name);
    }

    target_name
}

fn resolve_relative_to_main(main_journal: &Path, include: &str) -> PathBuf {
    let include_path = PathBuf::from(include);
    if include_path.is_absolute() {
        include_path
    } else {
        main_journal
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(include_path)
    }
}

fn glob_target_path(
    main_journal: &Path,
    includes: &[String],
    input: &TransactionInput,
) -> (PathBuf, String) {
    let year = input.date.chars().take(4).collect::<String>();
    let month = input.date.chars().skip(5).take(2).collect::<String>();
    let target_include = glob_target_include(includes, &year);
    let target_file = resolve_relative_to_main(
        main_journal,
        &target_include.replace("*.journal", &format!("{}.journal", month)),
    );
    (target_file, target_include)
}

fn glob_target_include(includes: &[String], year: &str) -> String {
    if let Some(existing) = includes
        .iter()
        .find(|include| glob_include_year(include).as_deref() == Some(year))
    {
        return existing.clone();
    }

    if let Some(first) = includes.first() {
        let mut parts = first.split('/').collect::<Vec<_>>();
        let _ = parts.pop();
        if let Some(index) = parts.iter().rposition(|part| {
            part.len() == 4 && part.chars().all(|character| character.is_ascii_digit())
        }) {
            parts[index] = year;
            return format!("{}/*.journal", parts.join("/"));
        }
    }

    format!("{}/*.journal", year)
}

fn glob_include_matches_year(target_include: &str, includes: &[String]) -> bool {
    let target_year = glob_include_year(target_include);
    includes
        .iter()
        .any(|include| glob_include_year(include) == target_year)
}

fn insert_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, parse_flat_include_filename)
}

fn insert_glob_include_sorted(content: &str, new_include: &str) -> String {
    insert_sorted_include(content, new_include, parse_glob_include)
}

fn insert_sorted_include<F>(content: &str, new_include: &str, parser: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
    let include_positions = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| parser(line).map(|include| (index, include)))
        .collect::<Vec<_>>();
    let new_line = format!("include {}", new_include);

    if include_positions.is_empty() {
        lines.push(new_line);
    } else {
        let insert_index = include_positions
            .iter()
            .find(|(_, include)| new_include < include.as_str())
            .map(|(index, _)| *index)
            .unwrap_or_else(|| {
                include_positions
                    .last()
                    .map(|(index, _)| index + 1)
                    .unwrap_or(lines.len())
            });
        lines.insert(insert_index, new_line);
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

/// Runs hledger check for the journal when hledger is available.
fn validate_journal(settings: &AppSettings, journal_path: &Path) -> Result<(), String> {
    let executable = hledger_executable(settings);
    let output = Command::new(&executable)
        .arg("-f")
        .arg(journal_path)
        .arg("check")
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(to_error_string_with_details(
            "hledger_check_failed",
            "hledger check failed. The journal may contain syntax errors.",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
        Err(error) => Err(to_error_string_with_details(
            "hledger_check_failed",
            "Unable to run hledger check.",
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::types::PostingInput;
    use serde_json::Value;

    fn posting(account: &str, amount: &str, commodity: &str) -> PostingInput {
        PostingInput {
            account: account.to_string(),
            amount: amount.to_string(),
            commodity: commodity.to_string(),
            unit_price: String::new(),
            comment: String::new(),
        }
    }

    fn valid_input() -> TransactionInput {
        TransactionInput {
            mode: "movement".to_string(),
            date: "2026-05-22".to_string(),
            status: "*".to_string(),
            code: "(INV-001)".to_string(),
            description: "Grocery store".to_string(),
            postings: vec![
                posting("expenses:food", "25.00", "EUR"),
                posting("assets:bank", "", ""),
            ],
        }
    }

    fn validation_error(input: TransactionInput) -> Value {
        let error = validate_transaction_input(&input).expect_err("input should be invalid");
        serde_json::from_str(&error).expect("validation error should be structured JSON")
    }

    fn field_error_paths(error: &Value) -> Vec<String> {
        let Some(field_errors) = error["fieldErrors"].as_array() else {
            return Vec::new();
        };

        field_errors
            .iter()
            .map(|field_error| {
                field_error["path"]
                    .as_array()
                    .map(|path| {
                        path.iter()
                            .filter_map(|part| part.as_str())
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn accepts_valid_transaction_input() {
        assert!(validate_transaction_input(&valid_input()).is_ok());
    }

    #[test]
    fn rejects_invalid_date_without_requiring_description() {
        let mut input = valid_input();
        input.date = "2026-02-30".to_string();
        input.description = "".to_string();

        let error = validation_error(input);
        let paths = field_error_paths(&error);

        assert_eq!(error["code"], "transaction_validation_failed");
        assert!(paths.contains(&"date".to_string()));
        assert!(!paths.contains(&"description".to_string()));
    }

    #[test]
    fn accepts_missing_description_in_all_modes() {
        for mode in ["movement", "investment", "advanced"] {
            let mut input = valid_input();
            input.mode = mode.to_string();
            input.description = "".to_string();

            assert!(validate_transaction_input(&input).is_ok());
        }
    }

    #[test]
    fn rejects_hledger_code_without_parentheses() {
        let mut input = valid_input();
        input.code = "INV-001".to_string();

        let paths = field_error_paths(&validation_error(input));

        assert!(paths.contains(&"code".to_string()));
    }

    #[test]
    fn rejects_transactions_without_two_posting_accounts() {
        let mut input = valid_input();
        input.postings = vec![posting("expenses:food", "25.00", "EUR")];

        let paths = field_error_paths(&validation_error(input));

        assert!(paths.contains(&"postings".to_string()));
    }

    #[test]
    fn rejects_unit_price_without_quantity() {
        let mut input = valid_input();
        input.postings[0].amount = "".to_string();
        input.postings[0].unit_price = "100 EUR".to_string();

        let paths = field_error_paths(&validation_error(input));

        assert!(paths.contains(&"postings.0.unitPrice".to_string()));
    }
}
