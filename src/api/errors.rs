//! API error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("HTTP status {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("API key not provided")]
    ApiKeyMissing,

    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Quota exceeded")]
    QuotaExceeded,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context length exceeded")]
    ContextLengthExceeded,

    #[error("API error: {0}")]
    ApiMessage(String),
}

impl ApiError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::HttpStatus { status: 429, .. }
        )
    }

    pub fn is_auth_error(&self) -> bool {
        matches!(self, Self::AuthenticationFailed | Self::ApiKeyMissing)
    }
}
