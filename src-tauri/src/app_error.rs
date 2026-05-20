use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured application error serialized as JSON over the Tauri bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl AppError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Serializes an AppError into a JSON string that the frontend can parse.
pub(crate) fn to_error_string(code: &str, message: impl Into<String>) -> String {
    let error = AppError::new(code, message);
    serde_json::to_string(&error)
        .unwrap_or_else(|_| format!(r#"{{"code":"{}","message":"Serialization failed"}}"#, code))
}

pub(crate) fn to_error_string_with_details(
    code: &str,
    message: impl Into<String>,
    details: impl Into<String>,
) -> String {
    let error = AppError::new(code, message).with_details(details);
    serde_json::to_string(&error)
        .unwrap_or_else(|_| format!(r#"{{"code":"{}","message":"Serialization failed"}}"#, code))
}
