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
pub(crate) fn parse_amount_style(files: &[JournalFile], _default_commodity: &str) -> AmountStyle {
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
                // format looks like: €1.000,00 or $1,234.56
                // Extract decimal mark (last non-digit char before the cents)
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
