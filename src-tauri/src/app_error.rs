use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured application error serialized as JSON over the Tauri bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FieldError {
    path: Vec<String>,
    message: String,
}

impl FieldError {
    pub(crate) fn new(
        path: impl IntoIterator<Item = impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into_iter().map(Into::into).collect(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    field_errors: Vec<FieldError>,
}

impl AppError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
            field_errors: Vec::new(),
        }
    }

    fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    fn with_field_errors(mut self, field_errors: Vec<FieldError>) -> Self {
        self.field_errors = field_errors;
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

pub(crate) fn to_validation_error_string(
    code: &str,
    message: impl Into<String>,
    field_errors: Vec<FieldError>,
) -> String {
    let error = AppError::new(code, message).with_field_errors(field_errors);
    serde_json::to_string(&error)
        .unwrap_or_else(|_| format!(r#"{{"code":"{}","message":"Serialization failed"}}"#, code))
}
