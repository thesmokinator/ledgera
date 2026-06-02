use crate::COMMODITY_STYLES;
use crate::{
    amount_format::{AmountFormatConfig, CommodityPosition},
    journal::files::JournalFile,
};
use serde::{Deserialize, Serialize};

/// Display style for amounts (decimal mark, digit grouping, precision, commodity placement).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AmountStyle {
    pub(crate) decimal_mark: String,
    pub(crate) digit_separator: String,
    pub(crate) digit_groups: Vec<usize>,
    pub(crate) precision: usize,
    pub(crate) commodity_position: String,
    pub(crate) commodity_spaced: bool,
}

impl Default for AmountStyle {
    fn default() -> Self {
        Self {
            decimal_mark: ".".to_string(),
            digit_separator: ",".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: "left".to_string(),
            commodity_spaced: false,
        }
    }
}

impl AmountStyle {
    pub(crate) fn from_hledger_json(bal: &serde_json::Value) -> Self {
        let style = &bal["astyle"];
        let decimal_mark = style["asdecimalmark"].as_str().unwrap_or(".").to_string();
        let digit_groups: Vec<usize> = style["asdigitgroups"]
            .as_array()
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_default();
        let digit_separator = style["asdigitgroups"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let precision = style["asprecision"].as_u64().unwrap_or(2) as usize;
        let commodity_position =
            parse_hledger_commodity_position(style).unwrap_or(CommodityPosition::Left);
        let commodity_spaced = parse_hledger_commodity_spaced(style).unwrap_or(false);
        Self {
            decimal_mark,
            digit_separator,
            digit_groups,
            precision,
            commodity_position: match commodity_position {
                CommodityPosition::Left => "left".to_string(),
                CommodityPosition::Right => "right".to_string(),
            },
            commodity_spaced,
        }
    }

    fn to_format_config(&self) -> AmountFormatConfig {
        AmountFormatConfig {
            decimal_mark: self.decimal_mark.clone(),
            digit_separator: self.digit_separator.clone(),
            digit_groups: self.digit_groups.clone(),
            precision: self.precision,
            commodity_position: if self.commodity_position == "right" {
                CommodityPosition::Right
            } else {
                CommodityPosition::Left
            },
            commodity_spaced: self.commodity_spaced,
        }
    }

    pub(crate) fn format(&self, amount: f64) -> String {
        self.to_format_config().format_quantity(amount)
    }

    pub(crate) fn format_amount(&self, amount: f64, commodity: &str) -> String {
        self.to_format_config().format_amount(amount, commodity)
    }
}

fn parse_hledger_commodity_position(style: &serde_json::Value) -> Option<CommodityPosition> {
    let value = style
        .get("ascommodityside")
        .or_else(|| style.get("ascommoditySide"))
        .or_else(|| style.get("commoditySide"))?;
    let side = value.as_str().unwrap_or_default().to_lowercase();
    if side.starts_with('r') {
        Some(CommodityPosition::Right)
    } else if side.starts_with('l') {
        Some(CommodityPosition::Left)
    } else {
        None
    }
}

fn parse_hledger_commodity_spaced(style: &serde_json::Value) -> Option<bool> {
    style
        .get("ascommodityspaced")
        .or_else(|| style.get("ascommoditySpaced"))
        .or_else(|| style.get("commoditySpaced"))
        .and_then(|value| value.as_bool())
}

/// Formats a number according to hledger display style (decimal mark, digit groups).
#[cfg(test)]
pub(crate) fn format_hledger_amount(amount: f64, bal: &serde_json::Value) -> String {
    AmountStyle::from_hledger_json(bal).format(amount)
}

/// Formats a number and commodity according to hledger display style.
pub(crate) fn format_hledger_display_amount(
    amount: f64,
    commodity: &str,
    bal: &serde_json::Value,
) -> String {
    AmountStyle::from_hledger_json(bal).format_amount(amount, commodity)
}

/// Parses the display style from commodity/format directives in journal files.
pub(crate) fn parse_amount_style(files: &[JournalFile]) -> AmountStyle {
    for file in files {
        let mut in_commodity = false;
        for line in file.content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("commodity ") {
                in_commodity = true;
                continue;
            }
            if in_commodity && trimmed.starts_with("format ") {
                let fmt = trimmed.strip_prefix("format ").unwrap_or("").trim();
                if let Some(style) = parse_format_directive(fmt) {
                    return style;
                }
                in_commodity = false;
            }
            if in_commodity && !line.starts_with(' ') && !trimmed.is_empty() {
                in_commodity = false;
            }
        }
    }
    AmountStyle::default()
}

/// Parses per-commodity display styles from all commodity/format directives.
pub(crate) fn parse_commodity_styles(
    files: &[JournalFile],
) -> std::collections::HashMap<String, AmountStyle> {
    let mut styles = std::collections::HashMap::new();
    let mut current_commodity: Option<String> = None;

    for file in files {
        for line in file.content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("commodity ") {
                // Extract commodity symbol from the sample amount
                let symbol = extract_commodity_symbol(rest.trim());
                current_commodity = Some(symbol);
                continue;
            }
            if let Some(fmt) = trimmed.strip_prefix("format ") {
                if let Some(ref commodity) = current_commodity {
                    if let Some(style) = parse_format_directive(fmt.trim()) {
                        styles.insert(commodity.clone(), style);
                    }
                }
                current_commodity = None;
                continue;
            }
            if current_commodity.is_some()
                && !line.starts_with(' ')
                && !trimmed.is_empty()
                && !trimmed.starts_with("format")
            {
                current_commodity = None;
            }
        }
    }

    styles
}

/// Extracts the commodity symbol from a sample amount like "€1.000,00" or "1,000.00 USD".
fn extract_commodity_symbol(sample: &str) -> String {
    let first_digit = sample.find(|c: char| c.is_ascii_digit());
    let last_digit = sample.rfind(|c: char| c.is_ascii_digit());

    let prefix = first_digit.map_or("", |i| &sample[..i]).trim();
    let suffix = last_digit.map_or("", |i| &sample[i + 1..]).trim();

    if !prefix.is_empty() {
        prefix.to_string()
    } else if !suffix.is_empty() {
        suffix.to_string()
    } else {
        sample.trim().to_string()
    }
}

pub(crate) fn parse_format_directive(fmt: &str) -> Option<AmountStyle> {
    let trimmed = fmt.trim();
    let first_digit = trimmed.find(|c: char| c.is_ascii_digit())?;
    let last_digit = trimmed.rfind(|c: char| c.is_ascii_digit())?;
    let prefix = &trimmed[..first_digit];
    let suffix = &trimmed[last_digit + 1..];
    let num_part = &trimmed[first_digit..=last_digit];
    let commodity_position = if !suffix.trim().is_empty() {
        "right"
    } else {
        "left"
    };
    let commodity_spaced = if commodity_position == "right" {
        suffix.chars().next().is_some_and(char::is_whitespace)
    } else {
        prefix.chars().last().is_some_and(char::is_whitespace)
    };

    let chars: Vec<char> = num_part.chars().collect();
    let len = chars.len();

    let mut trailing_digits = 0;
    for i in (0..len).rev() {
        if chars[i].is_ascii_digit() {
            trailing_digits += 1;
        } else {
            break;
        }
    }

    if trailing_digits == 0 || trailing_digits == len {
        let separator = chars
            .iter()
            .rev()
            .skip(trailing_digits)
            .find(|c| !c.is_ascii_digit())
            .map(|c| c.to_string())
            .unwrap_or_default();
        return Some(AmountStyle {
            decimal_mark: ".".to_string(),
            digit_separator: separator.clone(),
            digit_groups: if separator.is_empty() {
                vec![]
            } else {
                vec![3]
            },
            precision: 0,
            commodity_position: commodity_position.to_string(),
            commodity_spaced,
        });
    }

    let decimal_mark = chars[len - trailing_digits - 1].to_string();
    let int_part: String = chars[..len - trailing_digits - 1].iter().collect();
    let separator = int_part
        .chars()
        .rev()
        .find(|c| !c.is_ascii_digit())
        .map(|c| c.to_string())
        .unwrap_or_default();

    Some(AmountStyle {
        decimal_mark,
        digit_separator: separator.clone(),
        digit_groups: if separator.is_empty() {
            vec![]
        } else {
            vec![3]
        },
        precision: trailing_digits,
        commodity_position: commodity_position.to_string(),
        commodity_spaced,
    })
}

/// Returns the AmountStyle for a specific commodity, falling back to the global style.
pub(crate) fn style_for_commodity(commodity: &str) -> AmountStyle {
    explicit_style_for_commodity(commodity)
        .unwrap_or_else(|| crate::AMOUNT_STYLE.get().cloned().unwrap_or_default())
}

/// Returns the explicitly declared AmountStyle for a commodity, when present.
pub(crate) fn explicit_style_for_commodity(commodity: &str) -> Option<AmountStyle> {
    if let Some(styles) = COMMODITY_STYLES.get() {
        if let Some(style) = styles.get(commodity) {
            return Some(style.clone());
        }
    }
    None
}

/// ISO 4217 currency codes to common display symbols.
/// Only used when the journal doesn't already define its own commodity for that code.
static ISO_SYMBOLS: &[(&str, &str)] = &[
    ("EUR", "\u{20AC}"),
    ("USD", "$"),
    ("GBP", "\u{00A3}"),
    ("JPY", "\u{00A5}"),
    ("CHF", "CHF"),
    ("CAD", "CA$"),
    ("AUD", "A$"),
    ("NZD", "NZ$"),
    ("CNY", "\u{00A5}"),
    ("HKD", "HK$"),
    ("SGD", "S$"),
    ("SEK", "kr"),
    ("NOK", "kr"),
    ("DKK", "kr"),
    ("INR", "\u{20B9}"),
    ("RUB", "\u{20BD}"),
    ("BRL", "R$"),
    ("ZAR", "R"),
    ("TRY", "\u{20BA}"),
    ("KRW", "\u{20A9}"),
    ("PLN", "z\u{0142}"),
    ("THB", "\u{0E3F}"),
    ("MXN", "MX$"),
    ("ILS", "\u{20AA}"),
    ("PHP", "\u{20B1}"),
    ("CZK", "K\u{010D}"),
    ("HUF", "Ft"),
    ("RON", "lei"),
    ("IDR", "Rp"),
    ("MYR", "RM"),
    ("NGN", "\u{20A6}"),
];

/// Resolves an ISO currency code to the journal's preferred display symbol.
/// If the journal already defines a commodity with that code (e.g. `commodity 1.000,00 EUR`),
/// the code is kept as-is. Otherwise, a common symbol like € is used as fallback.
pub(crate) fn resolve_currency_display(iso_code: &str) -> String {
    if let Some(styles) = COMMODITY_STYLES.get() {
        if styles.contains_key(iso_code) {
            return iso_code.to_string();
        }
    }

    for (code, symbol) in ISO_SYMBOLS {
        if code.eq_ignore_ascii_case(iso_code) {
            return symbol.to_string();
        }
    }

    iso_code.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_euro_symbol_when_journal_does_not_define_eur() {
        let display = resolve_currency_display("EUR");
        assert_eq!(display, "\u{20AC}");
    }

    #[test]
    fn resolves_dollar_symbol() {
        let display = resolve_currency_display("USD");
        assert_eq!(display, "$");
    }

    #[test]
    fn resolves_pound_symbol() {
        let display = resolve_currency_display("GBP");
        assert_eq!(display, "\u{00A3}");
    }

    #[test]
    fn resolves_yen_symbol() {
        let display = resolve_currency_display("JPY");
        assert_eq!(display, "\u{00A5}");
    }

    #[test]
    fn resolves_swiss_franc_as_code() {
        let display = resolve_currency_display("CHF");
        assert_eq!(display, "CHF");
    }

    #[test]
    fn falls_back_to_code_for_unknown_currency() {
        let display = resolve_currency_display("XYZ");
        assert_eq!(display, "XYZ");
    }

    #[test]
    fn case_insensitive_lookup() {
        let display = resolve_currency_display("eur");
        assert_eq!(display, "\u{20AC}");
    }

    #[test]
    fn parses_commodity_styles_from_multi_currency_journal() {
        let files = &[JournalFile {
            path: "test.journal".into(),
            content: "commodity \u{20AC}1.000,00\n  format 1.000,00 \u{20AC}\n\ncommodity 1,000.00 USD\n  format 1,000.00 USD\n".to_string(),
        }];
        let styles = parse_commodity_styles(files);

        assert!(
            styles.contains_key("\u{20AC}"),
            "EUR symbol should be present"
        );
        assert!(styles.contains_key("USD"), "USD should be present");

        let eur_style = styles.get("\u{20AC}").unwrap();
        assert_eq!(eur_style.decimal_mark, ",");
        assert_eq!(eur_style.precision, 2);

        let usd_style = styles.get("USD").unwrap();
        assert_eq!(usd_style.decimal_mark, ".");
        assert_eq!(usd_style.precision, 2);
    }
}
