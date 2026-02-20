use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::{json, Value};

use crate::{
    db::payments as db,
    errors::AppError,
    models::payment::PaystackWebhookEvent,
    services::webhook::verify_paystack_signature,
    state::AppState,
};

/// POST /webhooks/paystack
///
/// Receives and processes Paystack webhook events.
///
/// Security checklist:
///   ✅ Signature verified with HMAC-SHA512 before any processing
///   ✅ Raw bytes used for verification (before any parsing)
///   ✅ Idempotency: duplicate events are silently skipped via DB unique constraint
///   ✅ Returns 200 immediately after validation to prevent Paystack retries
///   ✅ Heavy work spawned onto a background task
pub async fn handle_paystack_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // ── Step 1: Verify the signature BEFORE parsing anything ────────────────
    let signature = headers
        .get("x-paystack-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_paystack_signature(&state.config.paystack_webhook_secret, &body, signature) {
        tracing::warn!("Webhook received with invalid signature");
        return Err(AppError::InvalidSignature);
    }

    // ── Step 2: Parse the event ─────────────────────────────────────────────
    let event: PaystackWebhookEvent = serde_json::from_slice(&body)?;
    let raw_payload: Value = serde_json::from_slice(&body)?;

    // Use reference from data as our idempotency key (unique per transaction event)
    let event_id = event
        .data
        .get("reference")
        .and_then(|v| v.as_str())
        .map(|r| format!("{}:{}", event.event, r))
        .unwrap_or_else(|| format!("{}:{}", event.event, uuid::Uuid::new_v4()));

    tracing::info!(event = %event.event, event_id = %event_id, "Webhook received");

    // ── Step 3: Idempotency check — store event or return early if duplicate ─
    // This is an atomic insert with ON CONFLICT DO NOTHING in the DB layer.
    // Returns AppError::DuplicateEvent if already processed (which returns HTTP 200).
    db::insert_webhook_event(&state.db, &event_id, &event.event, &raw_payload).await?;

    // ── Step 4: Dispatch to the correct handler ──────────────────────────────
    // Clone what we need and spawn processing in the background so we can
    // immediately return 200 to Paystack (prevents their retry mechanism).
    let state_clone = state.clone();
    let event_id_clone = event_id.clone();

    tokio::spawn(async move {
        if let Err(e) = process_event(&state_clone, &event, &event_id_clone).await {
            tracing::error!(event_id = %event_id_clone, error = %e, "Failed to process webhook event");
        }
    });

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Webhook received" })),
    ))
}

/// Dispatches a verified, deduplicated webhook event to the right handler
async fn process_event(
    state: &AppState,
    event: &PaystackWebhookEvent,
    event_id: &str,
) -> Result<(), AppError> {
    match event.event.as_str() {
        "charge.success" => handle_charge_success(state, event).await?,
        "charge.failed" => handle_charge_failed(state, event).await?,
        "transfer.success" => handle_transfer_success(state, event).await?,
        "transfer.failed" => handle_transfer_failed(state, event).await?,
        "transfer.reversed" => handle_transfer_reversed(state, event).await?,
        other => {
            tracing::info!(event_type = %other, "Unhandled webhook event type — ignoring");
        }
    }

    // Mark as processed in DB after successful handling
    db::mark_webhook_processed(&state.db, event_id).await?;

    Ok(())
}

// ── Individual event handlers ────────────────────────────────────────────────

async fn handle_charge_success(
    state: &AppState,
    event: &PaystackWebhookEvent,
) -> Result<(), AppError> {
    let reference = event
        .data
        .get("reference")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing reference in charge.success".to_string()))?;

    tracing::info!(reference = %reference, "Processing charge.success");

    // Update payment status to "success"
    let payment = db::update_payment_status(&state.db, reference, "success").await?;

    tracing::info!(
        payment_id = %payment.id,
        reference = %reference,
        amount = payment.amount,
        "Payment marked as successful — fulfill order here"
    );

    // TODO: Add your business logic here, for example:
    //   - Send a confirmation email
    //   - Provision access / activate subscription
    //   - Update order status in your orders table
    //   - Emit an event to a queue

    Ok(())
}

async fn handle_charge_failed(
    state: &AppState,
    event: &PaystackWebhookEvent,
) -> Result<(), AppError> {
    let reference = event
        .data
        .get("reference")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing reference in charge.failed".to_string()))?;

    tracing::info!(reference = %reference, "Processing charge.failed");

    db::update_payment_status(&state.db, reference, "failed").await?;

    // TODO: Notify the customer, update your order status, etc.

    Ok(())
}

async fn handle_transfer_success(
    _state: &AppState,
    event: &PaystackWebhookEvent,
) -> Result<(), AppError> {
    let transfer_code = event
        .data
        .get("transfer_code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!(transfer_code = %transfer_code, "Transfer succeeded");
    // TODO: Update transfer record status in your DB

    Ok(())
}

async fn handle_transfer_failed(
    _state: &AppState,
    event: &PaystackWebhookEvent,
) -> Result<(), AppError> {
    let transfer_code = event
        .data
        .get("transfer_code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::warn!(transfer_code = %transfer_code, "Transfer failed");
    // TODO: Alert finance team, retry logic, etc.

    Ok(())
}

async fn handle_transfer_reversed(
    _state: &AppState,
    event: &PaystackWebhookEvent,
) -> Result<(), AppError> {
    let transfer_code = event
        .data
        .get("transfer_code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::warn!(transfer_code = %transfer_code, "Transfer reversed");
    // TODO: Reconcile your books

    Ok(())
}
