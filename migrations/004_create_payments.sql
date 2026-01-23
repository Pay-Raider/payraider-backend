-- 004_create_payments.sql

-- Stores normalized payment operations from Stellar
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    source_account TEXT NOT NULL,
    destination_account TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    asset_code TEXT,
    asset_issuer TEXT,
    amount NUMERIC NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Stores the last processed cursor for different ingestion tasks to ensure persistence and resumability
CREATE TABLE IF NOT EXISTS ingestion_state (
    task_name TEXT PRIMARY KEY,
    last_cursor TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
