use crate::app_error::to_error_string_with_details;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const DEFAULT_GIT_COMMIT_MESSAGE: &str = "Update journal from Ledgera";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppModuleSettings {
    #[serde(default)]
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitSyncModuleSettings {
    #[serde(default)]
    pub(crate) enabled: bool,
    #[serde(default = "default_git_commit_message")]
    pub(crate) commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppModulesSettings {
    #[serde(default)]
    pub(crate) market_prices: AppModuleSettings,
    #[serde(default)]
    pub(crate) developer_tools: AppModuleSettings,
    #[serde(default = "default_update_checker")]
    pub(crate) update_checker: AppModuleSettings,
    #[serde(default)]
    pub(crate) git_sync: GitSyncModuleSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommoditySymbolMapping {
    pub(crate) commodity: String,
    #[serde(rename = "yahooSymbol")]
    pub(crate) yahoo_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) journal_path: String,
    pub(crate) hledger_path: String,
    #[serde(default = "default_theme")]
    pub(crate) theme: String,
    #[serde(default = "default_language")]
    pub(crate) language: String,
    #[serde(default)]
    pub(crate) power_user: bool,
    #[serde(default)]
    pub(crate) default_commodity: String,
    #[serde(default)]
    pub(crate) fetch_prices: bool,
    #[serde(default)]
    pub(crate) commodity_symbols: Vec<CommoditySymbolMapping>,
    #[serde(default)]
    pub(crate) exclude_balances: Vec<String>,
    #[serde(default)]
    pub(crate) include_investments: Vec<String>,
    #[serde(default)]
    pub(crate) prefill_postings: bool,
    #[serde(default)]
    pub(crate) modules: AppModulesSettings,
}

impl Default for AppModuleSettings {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl Default for GitSyncModuleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            commit_message: default_git_commit_message(),
        }
    }
}

impl Default for AppModulesSettings {
    fn default() -> Self {
        Self {
            market_prices: AppModuleSettings::default(),
            developer_tools: AppModuleSettings::default(),
            update_checker: default_update_checker(),
            git_sync: GitSyncModuleSettings::default(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            journal_path: String::new(),
            hledger_path: String::new(),
            theme: default_theme(),
            language: default_language(),
            power_user: false,
            default_commodity: String::new(),
            fetch_prices: false,
            commodity_symbols: Vec::new(),
            exclude_balances: Vec::new(),
            include_investments: Vec::new(),
            prefill_postings: false,
            modules: AppModulesSettings::default(),
        }
    }
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "system".to_string()
}

fn default_git_commit_message() -> String {
    DEFAULT_GIT_COMMIT_MESSAGE.to_string()
}

fn default_update_checker() -> AppModuleSettings {
    AppModuleSettings { enabled: true }
}

/// Returns persisted application settings from the platform config directory.
#[tauri::command]
pub(crate) fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    read_settings(&app)
}

/// Persists application settings in the platform config directory.
#[tauri::command]
pub(crate) fn update_app_settings(
    app: AppHandle,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            to_error_string_with_details(
                "settings_save_failed",
                "Unable to create settings directory.",
                error.to_string(),
            )
        })?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| {
        to_error_string_with_details(
            "settings_save_failed",
            "Unable to encode settings.",
            error.to_string(),
        )
    })?;
    fs::write(&path, content).map_err(|error| {
        to_error_string_with_details(
            "settings_save_failed",
            "Unable to write settings file.",
            error.to_string(),
        )
    })?;
    Ok(settings)
}

pub(crate) fn read_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&path).map_err(|error| {
        to_error_string_with_details(
            "settings_read_failed",
            "Unable to read settings file.",
            error.to_string(),
        )
    })?;
    serde_json::from_str(&content).map_err(|error| {
        to_error_string_with_details(
            "settings_read_failed",
            "Settings file is corrupted.",
            error.to_string(),
        )
    })
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("settings.json"))
}
