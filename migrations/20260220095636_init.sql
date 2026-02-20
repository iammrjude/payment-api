-- Add migration script here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Stores payment records initiated by your API
CREATE TABLE payments (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reference       TEXT NOT NULL UNIQUE,   -- Provider's transaction reference
    email           TEXT NOT NULL,
    amount          BIGINT NOT NULL,         -- In smallest currency unit (kobo for NGN)
    currency        TEXT NOT NULL DEFAULT 'NGN',
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending | success | failed
    provider        TEXT NOT NULL DEFAULT 'paystack',
    checkout_url    TEXT,
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stores all incoming webhook events for idempotency + auditing
CREATE TABLE webhook_events (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_id        TEXT NOT NULL UNIQUE,    -- Provider's unique event identifier
    event_type      TEXT NOT NULL,           -- e.g. "charge.success"
    provider        TEXT NOT NULL DEFAULT 'paystack',
    payload         JSONB NOT NULL,
    processed       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payments_reference ON payments(reference);
CREATE INDEX idx_payments_status    ON payments(status);
CREATE INDEX idx_webhook_event_id   ON webhook_events(event_id);
