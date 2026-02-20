# payment-api

A production-ready Rust backend for integrating with **Paystack** (easily swappable for any provider), built with **Axum**, **Tokio**, **Serde**, **reqwest**, **sqlx**, **secrecy**, and **thiserror**.

**TODO:**

- For now I did not handle sending money.
- I only handled receiving money and the Webhook.

API Endpoint: <https://sclerosal-kacie-stalkingly.ngrok-free.dev>

Webhook URL: <https://sclerosal-kacie-stalkingly.ngrok-free.dev/webhooks/paystack>

This application is running locally on my machine and is exposed to the internet using ngrok. Ngrok creates a temporary public URL that tunnels requests to my local development server.

Because of this, the webhook URL is only accessible while the application is running on my machine and the ngrok tunnel is active. When the app is stopped or the ngrok session expires, the URL will no longer be reachable.

The webhook endpoint is used to receive event notifications (such as payment updates) from Paystack during development and testing.

---

## Features

- [x] Initiate payment sessions (returns Paystack checkout URL)
- [x] Query payment status by internal ID
- [x] Manual payment verification via Paystack API
- [x] Webhook handler with **HMAC-SHA512 signature verification**
- [x] **Idempotent** webhook processing (duplicate events safely skipped via DB)
- [x] Background event processing (returns `200` to Paystack immediately)
- [x] Handles: `charge.success`, `charge.failed`, `transfer.success`, `transfer.failed`, `transfer.reversed`
- [x] Secrets protected with `secrecy::SecretString`
- [x] Auto-migrations via `sqlx::migrate!`

---

## Project Structure

```text
payment-api/
├── migrations/
│   └── 20260220095636_init.sql     # payments + webhook_events tables
├── src/
│   ├── main.rs                     # App entry point, router, server
│   ├── config.rs                   # Env var loading with secrecy
│   ├── state.rs                    # Shared AppState (db + paystack + config)
│   ├── errors.rs                   # Unified AppError with IntoResponse
│   ├── models/
│   │   └── payment.rs              # Request/response/DB/Paystack structs
│   ├── db/
│   │   └── payments.rs             # All sqlx database queries
│   ├── services/
│   │   ├── payment.rs              # Paystack API client
│   │   └── webhook.rs              # HMAC-SHA512 signature verification
│   └── routes/
│       ├── payments.rs             # POST /payments, GET /payments/{id}, etc.
│       └── webhooks.rs             # POST /webhooks/paystack
├── .env.example
├── Cargo.toml
└── README.md
```

---

## Quickstart

### 1. Prerequisites

- Rust (stable)
- PostgreSQL running locally
- A [Paystack](https://paystack.com) account (free test keys available)

### 2. Clone & configure

```bash
cp .env.example .env
```

Edit `.env`:

```env
PAYSTACK_SECRET_KEY=sk_test_your_key_here
PAYSTACK_WEBHOOK_SECRET=your_webhook_secret
DATABASE_URL=postgres://postgres:password@localhost:5432/payment_db
PORT=3000
```

### 3. Create the database

```bash
# using createdb
createdb payment_db

# using psql
psql -U postgres -c "CREATE DATABASE payment_db;"

# Then inside psql, list all databases
\l
```

### 4. Run

```bash
# run `cargo sqlx prepare` to update the query cache
cargo sqlx prepare

# Then compile and run your project
cargo run
```

Migrations run automatically on startup.

### 5. Installing ngrok

```bash
# With Chocolatey
choco install ngrok

# Or with Winget
winget install ngrok.ngrok

# Or with WinGet via Microsoft Store
winget install ngrok -s msstore

# Then Setup
ngrok config add-authtoken your_auth_token

# Output
Authtoken saved to configuration file: C:\Users\your_name\AppData\Local/ngrok/ngrok.yml

# Deploy your app online
# ngrok http 80
ngrok http 3320

# You'll see output like:
Forwarding  https://a1b2c3d4.ngrok.io -> http://localhost:3320

# Go to your dev domain to see your app!
https://sclerosal-kacie-stalkingly.ngrok-free.
Forwarding  https://sclerosal-kacie-stalkingly.ngrok-free.dev -> http://localhost:3320
```

---

## API Reference

### `POST /payments`

Initiate a payment. Returns a Paystack checkout URL to redirect your user to.

**Request:**

```json
{
  "email": "customer@example.com",
  "amount": 5000,
  "currency": "NGN",
  "metadata": { "order_id": "ORD-123" }
}
```

> `amount` is in the smallest currency unit — **kobo** for NGN (5000 kobo = ₦50.00)

Example:

```bash
# Using curl on macOS / Linux / Git Bash

# Send NGN 25,000.00 to Samuel Achilike
curl -X POST http://localhost:3320/payments \
  -H "Content-Type: application/json" \
  -d '{
    "email": "samidope15@gmail.com",
    "amount": 2500000,
    "currency": "NGN",
    "metadata": { "order_id": "ORD-123" }
  }'
```

**Response `201`:**

```json
{
  "payment_id": "uuid-here",
  "reference": "PAY-abc123",
  "checkout_url": "https://checkout.paystack.com/...",
  "status": "pending"
}
```

---

### `GET /payments/{id}`

Get payment status by internal UUID.

**Response `200`:**

```json
{
  "payment_id": "uuid",
  "reference": "PAY-abc123",
  "email": "customer@example.com",
  "amount": 5000,
  "currency": "NGN",
  "status": "success",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:01:00Z"
}
```

---

### `GET /payments/{id}/verify`

Manually re-verify a payment directly with Paystack and sync status to DB.

---

### `POST /webhooks/paystack`

Receives Paystack webhook events. This endpoint must be:

- Registered in your [Paystack Dashboard](https://dashboard.paystack.com/#/settings/developer)
- Publicly accessible (use [ngrok](https://ngrok.com) for local testing)

**Events handled:**

| Event | Action |
| --- | --- |
| `charge.success` | Marks payment `success`, ready for order fulfillment |
| `charge.failed` | Marks payment `failed` |
| `transfer.success` | Logs transfer completion |
| `transfer.failed` | Logs transfer failure |
| `transfer.reversed` | Logs transfer reversal |

---

## Testing Webhooks Locally

```bash
# Install ngrok, then:
ngrok http 3000

# Set the ngrok URL as your webhook in Paystack dashboard:
# https://your-ngrok-url.ngrok.io/webhooks/paystack

# Simulate a webhook manually:
curl -X POST http://localhost:3000/webhooks/paystack \
  -H "Content-Type: application/json" \
  -H "x-paystack-signature: <computed-hmac>" \
  -d '{"event":"charge.success","data":{"reference":"PAY-test123","amount":5000,"currency":"NGN","customer":{"email":"test@example.com"}}}'
```

---

## Swapping to a Different Payment Provider

The architecture is designed so you only need to touch `src/services/payment.rs` and `src/services/webhook.rs`:

| Provider | Auth Header | Base URL | Webhook Header | HMAC Algorithm |
| --- | --- | --- | --- | --- |
| Paystack | `Bearer` | `api.paystack.co` | `x-paystack-signature` | SHA512 |
| Stripe | `Basic` | `api.stripe.com/v1` | `stripe-signature` | SHA256 + timestamp |
| Flutterwave | `Bearer` | `api.flutterwave.com/v3` | `verif-hash` | direct compare |
| PayPal | OAuth2 | `api-m.paypal.com` | `paypal-transmission-sig` | SHA256 |

---

## Security Notes

- Secrets are wrapped in `secrecy::SecretString` — never logged, only accessed via `.expose_secret()`
- Webhook signatures verified with constant-time HMAC comparison (timing attack resistant)
- Idempotency enforced at the DB level (`ON CONFLICT DO NOTHING`)
- Webhooks return `200` immediately; actual processing runs in a background `tokio::spawn`
- Duplicate events return `200` (not an error) so providers stop retrying
