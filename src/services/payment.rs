use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use crate::{
    errors::AppError,
    models::payment::{
        InitiatePaymentRequest, PaystackInitData, PaystackInitResponse, PaystackVerifyData,
        PaystackVerifyResponse,
    },
};

const PAYSTACK_BASE_URL: &str = "https://api.paystack.co";

pub struct PaystackService {
    client: Client,
    secret_key: SecretString,
}

impl PaystackService {
    pub fn new(secret_key: SecretString) -> Self {
        Self {
            client: Client::new(),
            secret_key,
        }
    }

    /// Call Paystack to create a new transaction and get a checkout URL.
    pub async fn initialize_transaction(
        &self,
        req: &InitiatePaymentRequest,
        reference: &str,
    ) -> Result<PaystackInitData, AppError> {
        let body = json!({
            "email":     req.email,
            "amount":    req.amount,        // already in kobo / smallest unit
            "currency":  req.currency,
            "reference": reference,
            "metadata":  req.metadata,
        });

        let response = self
            .client
            .post(format!("{}/transaction/initialize", PAYSTACK_BASE_URL))
            // ExposeSecret() is the explicit, auditable usage of the secret
            .bearer_auth(self.secret_key.expose_secret())
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();
        let parsed: PaystackInitResponse = response.json().await?;

        if !parsed.status {
            return Err(AppError::ProviderError {
                status,
                message: parsed.message,
            });
        }

        parsed.data.ok_or_else(|| AppError::ProviderError {
            status,
            message: "No data in Paystack initialize response".to_string(),
        })
    }

    /// Verify a transaction's status directly with Paystack (used after webhook or manual check).
    pub async fn verify_transaction(
        &self,
        reference: &str,
    ) -> Result<PaystackVerifyData, AppError> {
        let response = self
            .client
            .get(format!(
                "{}/transaction/verify/{}",
                PAYSTACK_BASE_URL, reference
            ))
            .bearer_auth(self.secret_key.expose_secret())
            .send()
            .await?;

        let status = response.status().as_u16();
        let parsed: PaystackVerifyResponse = response.json().await?;

        if !parsed.status {
            return Err(AppError::ProviderError {
                status,
                message: parsed.message,
            });
        }

        parsed.data.ok_or_else(|| AppError::ProviderError {
            status,
            message: "No data in Paystack verify response".to_string(),
        })
    }
}
