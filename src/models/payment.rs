use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── HTTP Request / Response models ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InitiatePaymentRequest {
    /// Customer email address
    pub email: String,
    /// Amount in smallest currency unit (kobo for NGN, cents for USD)
    pub amount: u64,
    /// ISO 4217 currency code e.g. "NGN", "USD"
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Optional extra data you want stored alongside the payment
    pub metadata: Option<serde_json::Value>,
}

fn default_currency() -> String {
    "NGN".to_string()
}

#[derive(Debug, Serialize)]
pub struct InitiatePaymentResponse {
    pub payment_id: Uuid,
    pub reference: String,
    pub checkout_url: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentStatusResponse {
    pub payment_id: Uuid,
    pub reference: String,
    pub email: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Database row model ───────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct PaymentRecord {
    pub id: Uuid,
    pub reference: String,
    pub email: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub provider: String,
    pub checkout_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Paystack API response shapes ─────────────────────────────────────────────

/// Shape of Paystack's POST /transaction/initialize response
#[derive(Debug, Deserialize)]
pub struct PaystackInitResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<PaystackInitData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaystackInitData {
    pub authorization_url: String,
    pub access_code: String,
    pub reference: String,
}

/// Shape of Paystack's GET /transaction/verify/:reference response
#[derive(Debug, Deserialize)]
pub struct PaystackVerifyResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<PaystackVerifyData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaystackVerifyData {
    pub reference: String,
    pub status: String, // "success" | "failed" | "abandoned"
    pub amount: u64,
    pub currency: String,
    pub customer: PaystackCustomer,
}

#[derive(Debug, Deserialize)]
pub struct PaystackCustomer {
    pub email: String,
}

// ── Webhook event models ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaystackWebhookEvent {
    pub event: String,                  // e.g. "charge.success"
    pub data: serde_json::Value,        // shape differs per event type
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct WebhookEventRecord {
    pub id: Uuid,
    pub event_id: String,
    pub event_type: String,
    pub provider: String,
    pub payload: serde_json::Value,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}
