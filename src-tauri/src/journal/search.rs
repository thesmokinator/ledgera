use crate::{
    load_journal_files, load_transactions_from_journal_via_files, require_journal_path,
    settings::read_settings, JournalTransaction,
};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalSearchResult {
    transaction: JournalTransaction,
    matches: Vec<JournalSearchMatch>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalSearchMatch {
    field: String,
    value: String,
    ranges: Vec<SearchMatchRange>,
    posting_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchMatchRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
struct ScoredJournalSearchResult {
    score: i32,
    result: JournalSearchResult,
}

/// Searches the configured journal and returns matching transactions.
#[tauri::command]
pub(crate) fn search_journal(
    app: AppHandle,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<JournalSearchResult>, String> {
    let settings = read_settings(&app)?;
    let journal_path = require_journal_path(&settings)?;
    let normalized_query = normalize_search_query(&query);
    if normalized_query.is_empty() {
        return Ok(Vec::new());
    }

    let files = load_journal_files(&journal_path)?;
    let transactions = load_transactions_from_journal_via_files(&files)?;
    let mut scored = transactions
        .into_iter()
        .filter_map(|transaction| search_journal_transaction(transaction, &normalized_query))
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.result.transaction.date.cmp(&a.result.transaction.date))
            .then_with(|| {
                b.result
                    .transaction
                    .start_line
                    .cmp(&a.result.transaction.start_line)
            })
    });

    Ok(scored
        .into_iter()
        .take(limit.unwrap_or(12))
        .map(|scored_result| scored_result.result)
        .collect())
}

fn normalize_search_query(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn score_search_match(haystack: &str, needle: &str) -> i32 {
    let normalized_haystack = normalize_search_query(haystack);
    let normalized_needle = normalize_search_query(needle);
    if normalized_needle.is_empty() || normalized_haystack.is_empty() {
        return 0;
    }

    if normalized_haystack == normalized_needle {
        return 100;
    }

    if normalized_haystack.starts_with(&normalized_needle) {
        return 80;
    }

    for word in normalized_haystack.split(|character: char| {
        character.is_whitespace() || matches!(character, '-' | '_' | ':' | '/')
    }) {
        if !word.is_empty() && word.starts_with(&normalized_needle) {
            return 60;
        }
    }

    if normalized_haystack.contains(&normalized_needle) {
        return 40;
    }

    let terms = normalized_needle.split_whitespace().collect::<Vec<_>>();
    if terms.len() > 1 && terms.iter().all(|term| normalized_haystack.contains(term)) {
        return 30;
    }

    0
}

fn search_journal_transaction(
    transaction: JournalTransaction,
    needle: &str,
) -> Option<ScoredJournalSearchResult> {
    let mut score = 0;
    let mut matches = Vec::new();

    add_search_match(
        &mut matches,
        &mut score,
        "description",
        &transaction.description,
        None,
        needle,
    );

    for (posting_index, posting) in transaction.postings.iter().enumerate() {
        add_search_match(
            &mut matches,
            &mut score,
            "account",
            &posting.account,
            Some(posting_index),
            needle,
        );
        add_search_match(
            &mut matches,
            &mut score,
            "comment",
            &posting.comment,
            Some(posting_index),
            needle,
        );
    }

    if score == 0 {
        return None;
    }

    Some(ScoredJournalSearchResult {
        score,
        result: JournalSearchResult {
            transaction,
            matches,
        },
    })
}

fn add_search_match(
    matches: &mut Vec<JournalSearchMatch>,
    score: &mut i32,
    field: &str,
    value: &str,
    posting_index: Option<usize>,
    needle: &str,
) {
    let match_score = score_search_match(value, needle);
    if match_score == 0 {
        return;
    }

    *score = (*score).max(match_score);
    matches.push(JournalSearchMatch {
        field: field.to_string(),
        value: value.to_string(),
        ranges: search_match_ranges(value, needle),
        posting_index,
    });
}

fn search_match_ranges(value: &str, needle: &str) -> Vec<SearchMatchRange> {
    let normalized_value = value.to_lowercase();
    let normalized_needle = normalize_search_query(needle);
    if normalized_value.is_empty() || normalized_needle.is_empty() {
        return Vec::new();
    }

    let terms = if normalized_needle.contains(' ') {
        normalized_needle.split_whitespace().collect::<Vec<_>>()
    } else {
        vec![normalized_needle.as_str()]
    };

    let mut ranges = Vec::new();
    for term in terms {
        let mut search_start = 0usize;
        while search_start < normalized_value.len() {
            let Some(relative_start) = normalized_value[search_start..].find(term) else {
                break;
            };
            let byte_start = search_start + relative_start;
            let byte_end = byte_start + term.len();
            ranges.push(SearchMatchRange {
                start: byte_to_char_index(value, byte_start),
                end: byte_to_char_index(value, byte_end),
            });
            search_start = byte_end;
        }
    }

    ranges.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.end.cmp(&b.end)));
    ranges.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    ranges
}

fn byte_to_char_index(value: &str, byte_index: usize) -> usize {
    value[..byte_index].chars().count()
}

#[cfg(test)]
mod tests {
    use super::{normalize_search_query, score_search_match, search_match_ranges};

    #[test]
    fn normalizes_search_queries() {
        assert_eq!(normalize_search_query("  Hello   World "), "hello world");
    }

    #[test]
    fn scores_like_matches_without_fuzzy_fallback() {
        assert_eq!(score_search_match("Tigros grocery", "tigros"), 80);
        assert_eq!(score_search_match("expenses:groceries", "gro"), 60);
        assert_eq!(score_search_match("weekly Lidl receipt", "lidl"), 60);
        assert_eq!(score_search_match("assets:bank", "lidl"), 0);
    }

    #[test]
    fn returns_highlight_ranges() {
        let ranges = search_match_ranges("Spesa Tigros Tigros", "tigros");
        assert_eq!(ranges.len(), 2);
        assert_eq!((ranges[0].start, ranges[0].end), (6, 12));
        assert_eq!((ranges[1].start, ranges[1].end), (13, 19));
    }
}
