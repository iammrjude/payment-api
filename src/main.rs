use axum::{
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use secrecy::ExposeSecret;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod config;
mod db;
mod errors;
mod models;
mod routes;
mod services;
mod state;

use config::AppConfig;
use routes::{
    payments::{get_payment_status, initiate_payment, verify_payment},
    webhooks::handle_paystack_webhook,
};
use state::AppState;

#[tokio::main]
async fn main() {
    // ── Logging ──────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // ── Config ───────────────────────────────────────────────────────────────
    let config = AppConfig::from_env();
    let addr = format!("{}:{}", config.host, config.port);

    tracing::info!("Starting payment-api on {}", addr);

    // ── Database ─────────────────────────────────────────────────────────────
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(config.database_url.expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    // Run any pending migrations automatically on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database connected and migrations applied");

    // ── App state ─────────────────────────────────────────────────────────────
    let state = AppState::new(pool, config);

    // ── Router ────────────────────────────────────────────────────────────────
    let app = Router::new()
        .route("/", get(root_handler))
        // Payment routes
        .route("/payments", post(initiate_payment))
        .route("/payments/{id}", get(get_payment_status))
        .route("/payments/{id}/verify", get(verify_payment))
        // Webhook routes
        .route("/webhooks/paystack", post(handle_paystack_webhook))
        // Health check
        .route("/health", get(health_check))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // Tighten this in production
        .with_state(state);

    // ── Serve ─────────────────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", addr));

    tracing::info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn root_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "message": "Welcome to the Payment API"
        })),
    )
}

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "message": "Server is running"
        })),
    )
}
