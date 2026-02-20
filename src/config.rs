use secrecy::SecretString;

/// All configuration loaded from environment variables at startup.
/// Secrets are wrapped in `SecretString` so they never appear in logs.
#[derive(Clone)]
pub struct AppConfig {
    pub paystack_secret_key: SecretString,
    pub paystack_webhook_secret: SecretString,
    pub database_url: SecretString,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        // Load .env file if present (no-op in production where env vars are set directly)
        dotenvy::dotenv().ok();

        Self {
            paystack_secret_key: SecretString::from(
                std::env::var("PAYSTACK_SECRET_KEY")
                    .expect("PAYSTACK_SECRET_KEY must be set"),
            ),
            paystack_webhook_secret: SecretString::from(
                std::env::var("PAYSTACK_WEBHOOK_SECRET")
                    .expect("PAYSTACK_WEBHOOK_SECRET must be set"),
            ),
            database_url: SecretString::from(
                std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            ),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a valid number"),
        }
    }
}
