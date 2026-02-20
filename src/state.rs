use sqlx::PgPool;
use std::sync::Arc;

use crate::{config::AppConfig, services::payment::PaystackService};

/// Shared application state — cloned cheaply into every Axum handler via Arc.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub paystack: Arc<PaystackService>,
    pub config: Arc<AppConfig>,
}

impl AppState {
    pub fn new(db: PgPool, config: AppConfig) -> Self {
        let paystack = Arc::new(PaystackService::new(config.paystack_secret_key.clone()));
        Self {
            db,
            paystack,
            config: Arc::new(config),
        }
    }
}
