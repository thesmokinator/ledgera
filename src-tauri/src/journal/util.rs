use chrono::NaiveDate;

/// Splits an inline hledger comment from a posting line.
pub(crate) fn split_inline_comment(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(';') {
        (&value[..index], value[index + 1..].trim())
    } else {
        (value, "")
    }
}

/// Splits the first token from a string.
pub(crate) fn split_first_token(value: &str) -> (&str, &str) {
    if let Some(index) = value.find(char::is_whitespace) {
        (&value[..index], &value[index..])
    } else {
        (value, "")
    }
}

/// Parses a journal date string in YYYY-MM-DD format.
pub(crate) fn parse_journal_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}
