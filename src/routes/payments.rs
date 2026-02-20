use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db::payments as db,
    errors::AppError,
    models::payment::{InitiatePaymentRequest, InitiatePaymentResponse, PaymentStatusResponse},
    state::AppState,
};

/// POST /payments
///
/// Initiates a new payment session with Paystack and returns a checkout URL
/// that the client should redirect the user to.
pub async fn initiate_payment(
    State(state): State<AppState>,
    Json(payload): Json<InitiatePaymentRequest>,
) -> Result<(StatusCode, Json<InitiatePaymentResponse>), AppError> {
    // Generate a unique reference for this transaction
    let reference = format!("PAY-{}", Uuid::new_v4().simple());

    tracing::info!(
        email = %payload.email,
        amount = payload.amount,
        reference = %reference,
        "Initiating payment"
    );

    // Call Paystack to create the transaction
    let paystack_data = state
        .paystack
        .initialize_transaction(&payload, &reference)
        .await?;

    // Persist the payment record in our database (status = "pending")
    let record = db::insert_payment(
        &state.db,
        &paystack_data.reference,
        &payload.email,
        payload.amount as i64,
        &payload.currency,
        &paystack_data.authorization_url,
        payload.metadata.as_ref(),
    )
    .await?;

    tracing::info!(
        reference = %record.reference,
        payment_id = %record.id,
        "Payment record created"
    );

    Ok((
        StatusCode::CREATED,
        Json(InitiatePaymentResponse {
            payment_id: record.id,
            reference: record.reference,
            checkout_url: paystack_data.authorization_url,
            status: record.status,
        }),
    ))
}

/// GET /payments/{id}
///
/// Returns the current status of a payment by its internal UUID.
pub async fn get_payment_status(
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> Result<Json<PaymentStatusResponse>, AppError> {
    let record = db::get_payment_by_id(&state.db, payment_id).await?;

    Ok(Json(PaymentStatusResponse {
        payment_id: record.id,
        reference: record.reference,
        email: record.email,
        amount: record.amount,
        currency: record.currency,
        status: record.status,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }))
}

/// GET /payments/{id}/verify
///
/// Re-verifies a payment directly with Paystack and syncs our DB.
/// Useful if a webhook was missed or for a manual reconciliation.
pub async fn verify_payment(
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let record = db::get_payment_by_id(&state.db, payment_id).await?;

    tracing::info!(reference = %record.reference, "Manually verifying payment with Paystack");

    let verified = state.paystack.verify_transaction(&record.reference).await?;

    // Sync status to DB if it changed
    if verified.status != record.status {
        db::update_payment_status(&state.db, &record.reference, &verified.status).await?;
        tracing::info!(
            reference = %record.reference,
            old_status = %record.status,
            new_status = %verified.status,
            "Payment status updated via manual verify"
        );
    }

    Ok(Json(json!({
        "payment_id": record.id,
        "reference":  verified.status,
        "status":     verified.status,
        "amount":     verified.amount,
        "currency":   verified.currency,
        "email":      verified.customer.email,
    })))
}
