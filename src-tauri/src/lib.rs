//! Application backend for Ledgera.
//!
//! The Rust layer owns journal access, conservative transaction edits, settings
//! persistence, and integration with the official hledger CLI.

mod accounts;
mod amount_format;
mod amount_style;
mod app_error;
mod balances;
mod budget;
mod git_crypt;
mod hledger;
mod investments;
mod journal;
mod logs;
mod reports;
mod settings;
mod sync;
mod updates;

use app_error::to_error_string_with_details;
use amount_style::AmountStyle;
#[cfg(test)]
use amount_style::{format_hledger_amount, format_hledger_display_amount, parse_format_directive};

#[cfg(test)]
use journal::autocomplete::{build_journal_profile, collect_declared_commodities};
#[cfg(test)]
use journal::parser::{format_posting, summarize_transaction};
#[cfg(test)]
use journal::types::{
    JournalPosting, JournalTransaction, PostingInput, TransactionDisplay, TransactionFlow,
    TransactionInput,
};
#[cfg(test)]
use journal::{
    files::JournalFile,
    summary::read_journal_summary,
    transactions::{
        create_transaction_for_settings, delete_transaction_for_settings,
        update_transaction_for_settings,
    },
};

#[cfg(test)]
use settings::AppSettings;
use std::sync::OnceLock;
#[cfg(test)]
use std::{
    env, fs,
    path::{Path, PathBuf},
};
static AMOUNT_STYLE: OnceLock<AmountStyle> = OnceLock::new();
static COMMODITY_STYLES: OnceLock<std::collections::HashMap<String, AmountStyle>> = OnceLock::new();

pub(crate) fn global_amount_style() -> &'static AmountStyle {
    AMOUNT_STYLE.get().unwrap_or_else(|| {
        static DEFAULT: OnceLock<AmountStyle> = OnceLock::new();
        DEFAULT.get_or_init(AmountStyle::default)
    })
}

/// Runs a blocking operation on a dedicated thread, translating join errors into structured errors.
pub(crate) async fn run_blocking<F, T>(code: &'static str, message: &'static str, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|error| to_error_string_with_details(code, message, error.to_string()))?
}

/// Returns a tint label for a numeric amount.
pub(crate) fn tint(amount: f64) -> &'static str {
    if amount < 0.0 {
        "negative"
    } else if amount > 0.0 {
        "positive"
    } else {
        "neutral"
    }
}

/// Starts the Tauri application and registers backend commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            logs::cleanup_old_logs(app.handle());

            let win_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                    .title("Ledgera")
                    .inner_size(1400.0, 918.0)
                    .min_inner_size(1080.0, 720.0);

            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(tauri::TitleBarStyle::Transparent);

            let _window = win_builder.build().unwrap();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_app_settings,
            settings::update_app_settings,
            hledger::check_hledger,
            sync::git_sync_status,
            sync::git_pull_journal,
            sync::git_commit_and_push_journal,
            updates::check_for_updates,
            journal::commands::list_transactions,
            journal::search::search_journal,
            journal::commands::create_transaction,
            journal::commands::update_transaction,
            journal::commands::delete_transaction,
            journal::periodic::list_periodic_rules,
            journal::periodic::create_periodic_rule,
            journal::periodic::update_periodic_rule,
            journal::periodic::delete_periodic_rule,
            journal::periodic::validate_period_expression,
            journal::periodic::compute_pending_recurring,
            journal::periodic::generate_recurring_transactions,
            logs::get_logs,
            logs::clear_logs,
            investments::get_investments_overview,
            reports::run_report,
            budget::run_budget_report,
            balances::get_balances,
            git_crypt::git_crypt_status,
            accounts::get_accounts_overview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("ledgera-{}-{}", name, nanos));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn settings_for(path: &Path) -> AppSettings {
        AppSettings {
            journal_path: path.to_string_lossy().to_string(),
            hledger_path: "true".to_string(),
            theme: "system".to_string(),
            language: "system".to_string(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: Vec::new(),
            exclude_balances: Vec::new(),
            include_investments: Vec::new(),
            prefill_postings: false,
            modules: Default::default(),
        }
    }

    fn posting(account: &str, amount: &str, commodity: &str) -> JournalPosting {
        JournalPosting {
            account: account.to_string(),
            amount: amount.to_string(),
            commodity: commodity.to_string(),
            unit_price: String::new(),
            comment: String::new(),
            raw: String::new(),
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should be created");
        }
        fs::write(path, content).expect("file should be written");
    }

    #[test]
    fn collects_only_declared_commodities() {
        let files = vec![JournalFile {
            path: PathBuf::from("main.journal"),
            content: "commodity €\n  format €1.000,00\ncommodity XEON\n\n2026-05-16 Opening balances\n    assets:investments:xeon  209 XEON @ €148,70087\n    equity:opening-balances\n"
                .to_string(),
        }];

        assert_eq!(collect_declared_commodities(&files), vec!["XEON", "€"]);
    }

    #[test]
    fn summarizes_income_flow_from_income_to_asset_with_implicit_posting() {
        let display = summarize_transaction(&[
            posting("assets:bank:postepay", "5.20", "€"),
            posting("income:other", "", ""),
        ]);

        assert_eq!(display.kind, "income");
        assert_eq!(display.flow.from, vec!["income:other"]);
        assert_eq!(display.flow.to, vec!["assets:bank:postepay"]);
    }

    #[test]
    fn summarizes_expense_flow_from_asset_to_expense_with_implicit_posting() {
        let display = summarize_transaction(&[
            posting("expenses:food", "20", "€"),
            posting("assets:bank", "", ""),
        ]);

        assert_eq!(display.kind, "expense");
        assert_eq!(display.flow.from, vec!["assets:bank"]);
        assert_eq!(display.flow.to, vec!["expenses:food"]);
    }

    #[test]
    fn summarizes_transfer_flow_from_negative_to_positive_posting() {
        let display = summarize_transaction(&[
            posting("assets:wallet", "-50", "€"),
            posting("assets:bank", "50", "€"),
        ]);

        assert_eq!(display.kind, "transfer");
        assert_eq!(display.flow.from, vec!["assets:wallet"]);
        assert_eq!(display.flow.to, vec!["assets:bank"]);
    }

    #[test]
    fn summarizes_asset_transfer_with_fee_by_main_asset_amount() {
        let display = summarize_transaction(&[
            posting("assets:bank:fineco", "-71", "€"),
            posting("expenses:fees", "1", "€"),
            posting("assets:cash", "70", "€"),
        ]);

        assert_eq!(display.kind, "transfer");
        assert_eq!(display.amount, "€70");
        assert_eq!(display.formatted, "€70.00");
        assert_eq!(display.flow.from, vec!["assets:bank:fineco"]);
        assert_eq!(display.flow.to, vec!["expenses:fees", "assets:cash"]);
    }

    #[test]
    fn summarizes_split_expense_flow_and_total_amount() {
        let display = summarize_transaction(&[
            posting("expenses:shopping", "26.58", "€"),
            posting("expenses:shopping:gifts", "19.99", "€"),
            posting("assets:bank:fineco", "", ""),
        ]);

        assert_eq!(display.kind, "expense");
        assert_eq!(display.amount, "-€46.57");
        assert_eq!(display.formatted, "€46.57");
        assert_eq!(display.flow.from, vec!["assets:bank:fineco"]);
        assert_eq!(
            display.flow.to,
            vec!["expenses:shopping", "expenses:shopping:gifts"]
        );
    }

    #[test]
    fn summarizes_opening_balance_amount_by_positive_commodities() {
        let display = summarize_transaction(&[
            posting("assets:bank:fineco", "5.706,51", "€"),
            posting("assets:bank:postepay", "890,05", "€"),
            posting("assets:investments:xeon", "209", "XEON @ €148,70087"),
            posting("equity:opening-balances", "", ""),
        ]);

        assert_eq!(display.kind, "transfer");
        assert_eq!(display.amount, "€6596.56 + 209 XEON");
        assert_eq!(display.formatted, "€6,596.56 + 209 XEON");
        assert_eq!(display.flow.from, vec!["equity:opening-balances"]);
        assert_eq!(
            display.flow.to,
            vec![
                "assets:bank:fineco",
                "assets:bank:postepay",
                "assets:investments:xeon"
            ]
        );
    }

    fn input(date: &str, description: &str) -> TransactionInput {
        TransactionInput {
            mode: "movement".to_string(),
            date: date.to_string(),
            status: "*".to_string(),
            code: String::new(),
            description: description.to_string(),
            postings: vec![
                PostingInput {
                    account: "expenses:test".to_string(),
                    amount: "10".to_string(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
                PostingInput {
                    account: "assets:cash".to_string(),
                    amount: String::new(),
                    commodity: "EUR".to_string(),
                    unit_price: String::new(),
                    comment: String::new(),
                },
            ],
        }
    }

    #[test]
    fn reads_transactions_from_flat_split_journal() {
        let dir = temp_dir("read-flat");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n\n2026-04-01 Main\n    assets:cash  1 EUR\n    equity:opening\n");
        write_file(
            &may,
            "2026-05-02 Split\n    expenses:office  10 EUR\n    assets:cash\n",
        );

        let summary = read_journal_summary(&main, "").expect("split journal should load");

        assert_eq!(summary.transactions.len(), 2);
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Split"
                && transaction.source_file.ends_with("2026-05.journal")));
    }

    #[test]
    fn reads_nested_tree_journal_with_directives_and_prices() {
        let dir = temp_dir("read-tree");
        let main = dir.join("main.journal");
        write_file(
            &main,
            "include accounts.journal\n\ninclude yearly/2026/recurring.journal\ninclude yearly/2026/2026-05.journal\n\ninclude prices/xeon.journal\n",
        );
        write_file(
            &dir.join("accounts.journal"),
            "commodity €\n  format €1.000,00\n\naccount assets:bank:fineco\n\n2026-05-16 Opening balances\n    assets:bank:fineco  €5706,51\n    equity:opening-balances\n",
        );
        write_file(
            &dir.join("yearly/2026/recurring.journal"),
            "; recurring transactions go here\n",
        );
        write_file(
            &dir.join("yearly/2026/2026-05.journal"),
            "; May 2026\n\n2026-05-20 Groceries\n    expenses:food  €25,00\n    assets:bank:fineco\n",
        );
        write_file(
            &dir.join("prices/xeon.journal"),
            "P 2026-05-16 XEON €148,70087\n",
        );

        let summary = read_journal_summary(&main, "").expect("tree journal should load");

        assert_eq!(summary.transactions.len(), 2);
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Opening balances"
                && transaction.source_file.ends_with("accounts.journal")));
        assert!(summary.transactions.iter().any(|transaction| {
            transaction.description == "Groceries" && transaction.display.formatted == "€25,00"
        }));
        assert!(summary
            .transactions
            .iter()
            .any(|transaction| transaction.description == "Groceries"
                && transaction
                    .source_file
                    .ends_with("yearly/2026/2026-05.journal")));
    }

    #[test]
    fn appends_to_existing_flat_subjournal_for_transaction_month() {
        let dir = temp_dir("append-flat-existing");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Existing\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-20", "New split transaction"))
            .expect("transaction should be routed to existing split file");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let may_content = fs::read_to_string(&may).expect("may journal should be readable");
        assert_eq!(main_content.matches("include 2026-05.journal").count(), 1);
        assert!(may_content.contains("2026-05-20 * New split transaction"));
    }

    #[test]
    fn appends_to_existing_tree_subjournal_for_transaction_month() {
        let dir = temp_dir("append-tree-existing");
        let main = dir.join("main.journal");
        let may = dir.join("yearly/2026/2026-05.journal");
        write_file(
            &main,
            "include accounts.journal\ninclude yearly/2026/2026-05.journal\ninclude prices/xeon.journal\n",
        );
        write_file(
            &dir.join("accounts.journal"),
            "account assets:bank:fineco\n",
        );
        write_file(
            &dir.join("prices/xeon.journal"),
            "P 2026-05-16 XEON €148,70087\n",
        );
        write_file(&may, "; May 2026\n");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-20", "Tree routed"))
            .expect("transaction should route to existing nested month file");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let may_content = fs::read_to_string(&may).expect("may journal should be readable");
        assert!(!main_content.contains("2026-05-20 * Tree routed"));
        assert!(may_content.contains("2026-05-20 * Tree routed"));
    }

    #[test]
    fn creates_missing_flat_tree_subjournal_and_sorted_include() {
        let dir = temp_dir("append-tree-new");
        let main = dir.join("main.journal");
        write_file(
            &main,
            "include yearly/2026/2026-04.journal\ninclude yearly/2026/2026-06.journal\n",
        );
        write_file(&dir.join("yearly/2026/2026-04.journal"), "");
        write_file(&dir.join("yearly/2026/2026-06.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-03", "Inserted tree month"))
            .expect("missing nested month journal should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let includes = main_content
            .lines()
            .filter(|line| line.starts_with("include"))
            .collect::<Vec<_>>();
        assert_eq!(
            includes,
            vec![
                "include yearly/2026/2026-04.journal",
                "include yearly/2026/2026-05.journal",
                "include yearly/2026/2026-06.journal",
            ]
        );
        assert!(fs::read_to_string(dir.join("yearly/2026/2026-05.journal"))
            .expect("new nested split file should exist")
            .contains("2026-05-03 * Inserted tree month"));
    }

    #[test]
    fn creates_missing_flat_subjournal_and_sorted_include() {
        let dir = temp_dir("append-flat-new");
        let main = dir.join("main.journal");
        write_file(&main, "include 2026-04.journal\ninclude 2026-06.journal\n");
        write_file(&dir.join("2026-04.journal"), "");
        write_file(&dir.join("2026-06.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-05-03", "Inserted month"))
            .expect("missing month journal should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        let includes = main_content
            .lines()
            .filter(|line| line.starts_with("include"))
            .collect::<Vec<_>>();
        assert_eq!(
            includes,
            vec![
                "include 2026-04.journal",
                "include 2026-05.journal",
                "include 2026-06.journal",
            ]
        );
        assert!(fs::read_to_string(dir.join("2026-05.journal"))
            .expect("new split file should exist")
            .contains("2026-05-03 * Inserted month"));
    }

    #[test]
    fn creates_new_nested_glob_year_and_month_file() {
        let dir = temp_dir("append-nested-glob-new-year");
        let main = dir.join("main.journal");
        write_file(&main, "include yearly/2025/*.journal\n");
        write_file(&dir.join("yearly/2025/12.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-01-15", "New nested glob year"))
            .expect("new nested glob year should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        assert!(main_content.contains("include yearly/2026/*.journal"));
        assert!(fs::read_to_string(dir.join("yearly/2026/01.journal"))
            .expect("new nested glob month should exist")
            .contains("2026-01-15 * New nested glob year"));
    }

    #[test]
    fn creates_new_glob_year_and_month_file() {
        let dir = temp_dir("append-glob-new-year");
        let main = dir.join("main.journal");
        write_file(&main, "include 2025/*.journal\n");
        write_file(&dir.join("2025/12.journal"), "");
        let settings = settings_for(&main);

        create_transaction_for_settings(&settings, &input("2026-01-15", "New glob year"))
            .expect("new glob year should be created");

        let main_content = fs::read_to_string(&main).expect("main should be readable");
        assert!(main_content.contains("include 2026/*.journal"));
        assert!(fs::read_to_string(dir.join("2026/01.journal"))
            .expect("new glob month should exist")
            .contains("2026-01-15 * New glob year"));
    }

    #[test]
    fn update_and_delete_target_source_subjournal() {
        let dir = temp_dir("mutate-source");
        let main = dir.join("main.journal");
        let may = dir.join("2026-05.journal");
        write_file(&main, "include 2026-05.journal\n");
        write_file(
            &may,
            "2026-05-02 Original\n    expenses:office  10 EUR\n    assets:cash\n",
        );
        let settings = settings_for(&main);
        let summary = read_journal_summary(&main, "").expect("summary should load");
        let transaction_id = summary.transactions[0].id.clone();

        update_transaction_for_settings(
            &settings,
            &transaction_id,
            &input("2026-05-02", "Updated"),
        )
        .expect("source subjournal transaction should update");
        let updated_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(updated_content.contains("2026-05-02 * Updated"));
        assert!(!updated_content.contains("Original"));

        let updated_summary = read_journal_summary(&main, "").expect("updated summary should load");
        delete_transaction_for_settings(&settings, &updated_summary.transactions[0].id)
            .expect("source subjournal transaction should delete");
        let deleted_content = fs::read_to_string(&may).expect("may should be readable");
        assert!(!deleted_content.contains("2026-05-02"));
    }

    #[test]
    fn builds_frequency_based_journal_profile() {
        let transactions = vec![
            JournalTransaction {
                id: "test:1".to_string(),
                source_file: "test".to_string(),
                date: "2026-05-16".to_string(),
                status: String::new(),
                code: String::new(),
                description: "Groceries".to_string(),
                postings: vec![
                    JournalPosting {
                        account: "expenses:food".to_string(),
                        amount: "25".to_string(),
                        commodity: "€".to_string(),
                        unit_price: String::new(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                    JournalPosting {
                        account: "assets:bank:fineco".to_string(),
                        amount: String::new(),
                        commodity: String::new(),
                        unit_price: String::new(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                ],
                display: TransactionDisplay {
                    account: "expenses:food".to_string(),
                    amount: "-€25".to_string(),
                    formatted: "-25,00".to_string(),
                    kind: "expense".to_string(),
                    tint: "negative".to_string(),
                    flow: TransactionFlow {
                        from: vec!["assets:bank:fineco".to_string()],
                        to: vec!["expenses:food".to_string()],
                    },
                },
                raw: String::new(),
                start_line: 1,
                end_line: 3,
            },
            JournalTransaction {
                id: "test:2".to_string(),
                source_file: "test".to_string(),
                date: "2026-05-17".to_string(),
                status: String::new(),
                code: String::new(),
                description: "Buy fund".to_string(),
                postings: vec![
                    JournalPosting {
                        account: "assets:investments:xeon".to_string(),
                        amount: "10".to_string(),
                        commodity: "XEON".to_string(),
                        unit_price: String::new(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                    JournalPosting {
                        account: "assets:bank:fineco".to_string(),
                        amount: "1487".to_string(),
                        commodity: "€".to_string(),
                        unit_price: String::new(),
                        comment: String::new(),
                        raw: String::new(),
                    },
                ],
                display: TransactionDisplay {
                    account: "assets:investments:xeon".to_string(),
                    amount: "10 XEON".to_string(),
                    formatted: "10 XEON".to_string(),
                    kind: "transfer".to_string(),
                    tint: "positive".to_string(),
                    flow: TransactionFlow {
                        from: Vec::new(),
                        to: vec![
                            "assets:investments:xeon".to_string(),
                            "assets:bank:fineco".to_string(),
                        ],
                    },
                },
                raw: String::new(),
                start_line: 5,
                end_line: 7,
            },
        ];

        let profile = build_journal_profile(&transactions, "€");

        assert_eq!(profile.default_cash_account, "assets:bank:fineco");
        assert_eq!(profile.default_expense_account, "expenses:food");
        assert_eq!(
            profile.default_investment_account,
            "assets:investments:xeon"
        );
        assert_eq!(profile.default_investment_commodity, "XEON");
    }

    #[test]
    fn formats_amount_with_italian_locale() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ",",
                "asdigitgroups": [".", [3]],
                "asprecision": 2
            }
        });
        assert_eq!(format_hledger_amount(33452.31, &bal), "33.452,31");
        assert_eq!(format_hledger_amount(-1858.0, &bal), "-1.858,00");
        assert_eq!(format_hledger_amount(2303.51, &bal), "2.303,51");
    }

    #[test]
    fn formats_amount_with_english_locale() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": [",", [3]],
                "asprecision": 2
            }
        });
        assert_eq!(format_hledger_amount(1234567.89, &bal), "1,234,567.89");
    }

    #[test]
    fn formats_amount_without_digit_groups() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": null,
                "asprecision": 0
            }
        });
        assert_eq!(format_hledger_amount(209.0, &bal), "209");
    }

    #[test]
    fn parses_right_side_commodity_from_format_directive() {
        let style = parse_format_directive("1.000,00 €").expect("format directive should parse");

        assert_eq!(style.commodity_position, "right");
        assert!(style.commodity_spaced);
        assert_eq!(style.format_amount(5317.55, "€"), "5.317,55 €");
    }

    #[test]
    fn parses_left_side_commodity_from_format_directive() {
        let style = parse_format_directive("$1,000.00").expect("format directive should parse");

        assert_eq!(style.commodity_position, "left");
        assert!(!style.commodity_spaced);
        assert_eq!(style.format_amount(5317.55, "$"), "$5,317.55");
    }

    #[test]
    fn formats_display_amount_with_right_side_commodity_style() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ",",
                "asdigitgroups": [".", [3]],
                "asprecision": 2,
                "ascommodityside": "R",
                "ascommodityspaced": true
            }
        });
        assert_eq!(
            format_hledger_display_amount(5317.55, "€", &bal),
            "5.317,55 €"
        );
        assert_eq!(
            format_hledger_display_amount(-37744.98, "€", &bal),
            "-37.744,98 €"
        );
    }

    #[test]
    fn formats_display_amount_with_left_side_commodity_style() {
        let bal = serde_json::json!({
            "astyle": {
                "asdecimalmark": ".",
                "asdigitgroups": [",", [3]],
                "asprecision": 2,
                "ascommodityside": "L",
                "ascommodityspaced": false
            }
        });
        assert_eq!(
            format_hledger_display_amount(1234.56, "$", &bal),
            "$1,234.56"
        );
        assert_eq!(
            format_hledger_display_amount(-1234.56, "$", &bal),
            "-$1,234.56"
        );
    }

    #[test]
    fn formats_posting_with_unit_price() {
        let posting = PostingInput {
            account: "assets:investments:etf".to_string(),
            amount: "10".to_string(),
            commodity: "VWCE".to_string(),
            unit_price: "150 EUR".to_string(),
            comment: String::new(),
        };
        let result = format_posting(&posting);
        assert!(result.contains("@ 150 EUR"));
        assert!(result.contains("10.00 VWCE"));
        assert!(result.contains("assets:investments:etf"));
    }

    #[test]
    fn formats_posting_without_unit_price_when_empty() {
        let posting = PostingInput {
            account: "expenses:food".to_string(),
            amount: "25".to_string(),
            commodity: "EUR".to_string(),
            unit_price: String::new(),
            comment: String::new(),
        };
        let result = format_posting(&posting);
        assert!(!result.contains('@'));
        assert!(result.contains("25.00 EUR"));
    }

    #[test]
    fn formats_posting_with_unit_price_and_comment() {
        let posting = PostingInput {
            account: "assets:investments:etf".to_string(),
            amount: "5".to_string(),
            commodity: "BTC".to_string(),
            unit_price: "45000 USD".to_string(),
            comment: "limit order".to_string(),
        };
        let result = format_posting(&posting);
        assert!(result.contains("@ 45000 USD"));
        assert!(result.contains("; limit order"));
    }

    #[test]
    fn tint_returns_negative_for_negative_amount() {
        assert_eq!(tint(-1.0), "negative");
        assert_eq!(tint(-0.01), "negative");
        assert_eq!(tint(f64::MIN), "negative");
    }

    #[test]
    fn tint_returns_positive_for_positive_amount() {
        assert_eq!(tint(1.0), "positive");
        assert_eq!(tint(0.01), "positive");
        assert_eq!(tint(f64::MAX), "positive");
    }

    #[test]
    fn tint_returns_neutral_for_zero() {
        assert_eq!(tint(0.0), "neutral");
        assert_eq!(tint(-0.0), "neutral");
    }

    #[test]
    fn z_aggregates_multiple_amount_lots_for_same_account() {
        let json = r#"[
  [
    [
      "assets:investments:xeon",
      "assets:investments:xeon",
      0,
      [
        {
          "acommodity": "XEON",
          "aquantity": { "floatingPoint": 209.0 },
          "astyle": { "ascommodityside": "R", "ascommodityspaced": true, "asdecimalmark": ",", "asdigitgroups": null, "asprecision": 2, "asrounding": "HardRounding" }
        },
        {
          "acommodity": "XEON",
          "aquantity": { "floatingPoint": -205.0 },
          "astyle": { "ascommodityside": "R", "ascommodityspaced": true, "asdecimalmark": ",", "asdigitgroups": null, "asprecision": 2, "asrounding": "HardRounding" }
        }
      ]
    ]
  ],
  [ { }, { } ]
]"#;
        let settings = AppSettings {
            journal_path: "test".to_string(),
            hledger_path: "true".to_string(),
            ..Default::default()
        };
        let result = balances::parse_balance_output(json, &settings, false).unwrap();
        assert_eq!(result.len(), 1, "should aggregate multiple lots into one entry");
        let balance = &result[0];
        assert_eq!(balance.account, "assets:investments:xeon");
        assert_eq!(balance.amount, 4.0);
        assert_eq!(balance.commodity, "XEON");
    }
}
