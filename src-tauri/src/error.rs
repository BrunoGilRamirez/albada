use serde::Serialize;

/// Stable error categories exchanged across the native command boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Validation,
    NotFound,
    Database,
    #[serde(rename = "IO")]
    Io,
    Unknown,
}

/// Safe error payload for frontend consumers.
///
/// Messages are intentionally safe and do not contain operation details that
/// could expose filesystem paths, queries, or other native implementation data.
/// A structured `detail` may carry small, domain-safe data such as the task IDs
/// involved in a rejected cycle.
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Validation, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Database, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Io, message)
    }

    pub fn unknown() -> Self {
        Self::new(ErrorCode::Unknown, "An unexpected error occurred.")
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::not_found("The requested item was not found.")
            }
            other => {
                log::error!("SQLite error: {other}");
                AppError::database("The local data operation could not be completed.")
            }
        }
    }
}

impl From<uuid::Error> for AppError {
    fn from(_: uuid::Error) -> Self {
        AppError::validation("The provided identifier is invalid.")
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        log::error!("I/O error: {err}");
        AppError::io("The local file operation could not be completed.")
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        log::error!("Tauri error: {err}");
        AppError::io("The local application operation could not be completed.")
    }
}

impl From<log::SetLoggerError> for AppError {
    fn from(_: log::SetLoggerError) -> Self {
        AppError::unknown()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}
