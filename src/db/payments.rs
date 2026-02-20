use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, models::payment::PaymentRecord};

/// Insert a new payment record (status = "pending")
pub async fn insert_payment(
    pool: &PgPool,
    reference: &str,
    email: &str,
    amount: i64,
    currency: &str,
    checkout_url: &str,
    metadata: Option<&serde_json::Value>,
) -> Result<PaymentRecord, AppError> {
    let record = sqlx::query_as::<_, PaymentRecord>(
        r#"
        INSERT INTO payments (reference, email, amount, currency, checkout_url, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(reference)
    .bind(email)
    .bind(amount)
    .bind(currency)
    .bind(checkout_url)
    .bind(metadata)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

/// Fetch a single payment by its internal UUID
pub async fn get_payment_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<PaymentRecord, AppError> {
    let record = sqlx::query_as::<_, PaymentRecord>(
        "SELECT * FROM payments WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Payment {} not found", id)))?;

    Ok(record)
}

/// Fetch a payment by provider reference string
#[allow(dead_code)]
pub async fn get_payment_by_reference(
    pool: &PgPool,
    reference: &str,
) -> Result<Option<PaymentRecord>, AppError> {
    let record = sqlx::query_as::<_, PaymentRecord>(
        "SELECT * FROM payments WHERE reference = $1",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// Update a payment's status (e.g. "pending" → "success" or "failed")
pub async fn update_payment_status(
    pool: &PgPool,
    reference: &str,
    status: &str,
) -> Result<PaymentRecord, AppError> {
    let record = sqlx::query_as::<_, PaymentRecord>(
        r#"
        UPDATE payments
        SET status = $1, updated_at = NOW()
        WHERE reference = $2
        RETURNING *
        "#,
    )
    .bind(status)
    .bind(reference)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "Payment with reference '{}' not found",
            reference
        ))
    })?;

    Ok(record)
}

/// Insert a webhook event. Returns Err if the event_id already exists (duplicate).
pub async fn insert_webhook_event(
    pool: &PgPool,
    event_id: &str,
    event_type: &str,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        INSERT INTO webhook_events (event_id, event_type, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(event_type)
    .bind(payload)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::DuplicateEvent);
    }

    Ok(())
}

/// Mark a webhook event as processed
pub async fn mark_webhook_processed(
    pool: &PgPool,
    event_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE webhook_events SET processed = TRUE WHERE event_id = $1",
    )
    .bind(event_id)
    .execute(pool)
    .await?;

    Ok(())
}
