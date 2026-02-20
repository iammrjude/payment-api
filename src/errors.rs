use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // ── Payment provider errors ──────────────────────────────────────────────
    #[error("HTTP request to payment provider failed: {0}")]
    ProviderRequest(#[from] reqwest::Error),

    #[error("Payment provider returned error {status}: {message}")]
    ProviderError { status: u16, message: String },

    // ── Webhook errors ───────────────────────────────────────────────────────
    #[error("Invalid webhook signature")]
    InvalidSignature,

    #[error("Webhook event already processed (idempotent skip)")]
    DuplicateEvent,

    // ── Serialization ────────────────────────────────────────────────────────
    #[error("Failed to parse payload: {0}")]
    Deserialization(#[from] serde_json::Error),

    // ── Database ─────────────────────────────────────────────────────────────
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // ── Generic ──────────────────────────────────────────────────────────────
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

/// Convert AppError → HTTP response automatically so handlers can use `?`
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InvalidSignature => (StatusCode::UNAUTHORIZED, self.to_string()),

            // Return 200 to stop provider retrying — we already handled this event
            AppError::DuplicateEvent => {
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "duplicate event, skipped" })),
                )
                    .into_response()
            }

            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),

            // Don't leak internal details to the client
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::ProviderRequest(e) => {
                tracing::error!("Provider request error: {:?}", e);
                (StatusCode::BAD_GATEWAY, "Payment provider unreachable".to_string())
            }
            AppError::ProviderError { status, message } => {
                tracing::error!("Provider error {}: {}", status, message);
                (StatusCode::BAD_GATEWAY, format!("Provider error: {}", message))
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
