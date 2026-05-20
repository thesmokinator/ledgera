#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommodityPosition {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmountFormatConfig {
    pub decimal_mark: String,
    pub digit_separator: String,
    pub digit_groups: Vec<usize>,
    pub precision: usize,
    pub commodity_position: CommodityPosition,
    pub commodity_spaced: bool,
}

impl Default for AmountFormatConfig {
    fn default() -> Self {
        Self {
            decimal_mark: ".".to_string(),
            digit_separator: ",".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: CommodityPosition::Left,
            commodity_spaced: false,
        }
    }
}

impl AmountFormatConfig {
    pub fn format_quantity(&self, amount: f64) -> String {
        let abs = amount.abs();
        let sign = if amount < 0.0 { "-" } else { "" };
        let formatted_num = format!("{:.*}", self.precision, abs);
        let parts: Vec<&str> = formatted_num.split('.').collect();
        let int_part = parts[0];
        let frac_part = parts.get(1).unwrap_or(&"");

        let grouped_int = if self.digit_groups.is_empty() || self.digit_separator.is_empty() {
            int_part.to_string()
        } else {
            let mut result = String::new();
            let mut remaining = int_part.len();
            let mut group_iter = self.digit_groups.iter().cycle();
            let mut first = true;
            while remaining > 0 {
                if !first {
                    result.insert_str(0, &self.digit_separator);
                }
                first = false;
                let group_size = group_iter.next().unwrap_or(&3);
                let take = (*group_size).min(remaining);
                let start = remaining - take;
                result.insert_str(0, &int_part[start..remaining]);
                remaining = start;
            }
            result
        };

        let decimal_part = if frac_part.is_empty() {
            String::new()
        } else {
            format!("{}{}", self.decimal_mark, frac_part)
        };

        format!("{}{}{}", sign, grouped_int, decimal_part)
    }

    pub fn format_amount(&self, amount: f64, commodity: &str) -> String {
        let commodity = commodity.trim();
        let quantity = self.format_quantity(amount);
        if commodity.is_empty() {
            return quantity;
        }

        let sign = if quantity.starts_with('-') { "-" } else { "" };
        let absolute_quantity = quantity.trim_start_matches('-');
        let separator = if self.commodity_spaced { " " } else { "" };

        match self.commodity_position {
            CommodityPosition::Left => {
                format!("{}{}{}{}", sign, commodity, separator, absolute_quantity)
            }
            CommodityPosition::Right => {
                format!("{}{}{}{}", sign, absolute_quantity, separator, commodity)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AmountFormatConfig, CommodityPosition};

    #[test]
    fn formats_quantity_with_decimal_and_digit_groups() {
        let config = AmountFormatConfig {
            decimal_mark: ",".to_string(),
            digit_separator: ".".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: CommodityPosition::Right,
            commodity_spaced: true,
        };

        assert_eq!(config.format_quantity(5317.55), "5.317,55");
        assert_eq!(config.format_quantity(-37744.98), "-37.744,98");
    }

    #[test]
    fn formats_right_side_spaced_commodity() {
        let config = AmountFormatConfig {
            decimal_mark: ",".to_string(),
            digit_separator: ".".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: CommodityPosition::Right,
            commodity_spaced: true,
        };

        assert_eq!(config.format_amount(5317.55, "€"), "5.317,55 €");
        assert_eq!(config.format_amount(-37744.98, "€"), "-37.744,98 €");
    }

    #[test]
    fn formats_left_side_unspaced_commodity() {
        let config = AmountFormatConfig {
            decimal_mark: ".".to_string(),
            digit_separator: ",".to_string(),
            digit_groups: vec![3],
            precision: 2,
            commodity_position: CommodityPosition::Left,
            commodity_spaced: false,
        };

        assert_eq!(config.format_amount(1234.56, "$"), "$1,234.56");
        assert_eq!(config.format_amount(-1234.56, "$"), "-$1,234.56");
    }

    #[test]
    fn formats_amount_without_commodity_as_quantity_only() {
        let config = AmountFormatConfig::default();

        assert_eq!(config.format_amount(42.0, ""), "42.00");
    }
}
