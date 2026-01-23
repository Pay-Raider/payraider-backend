-- I'm creating tables for ledger ingestion as specified in issue #2

-- Ledgers table: stores ingested ledger metadata
CREATE TABLE IF NOT EXISTS ledgers (
    sequence BIGINT PRIMARY KEY,
    hash VARCHAR(64) NOT NULL,
    close_time TIMESTAMPTZ NOT NULL,
    transaction_count INTEGER NOT NULL DEFAULT 0,
    operation_count INTEGER NOT NULL DEFAULT 0,
    ingested_at TIMESTAMPTZ DEFAULT NOW()
);

-- Transactions table: stores transaction info from ledgers
CREATE TABLE IF NOT EXISTS transactions (
    hash VARCHAR(64) PRIMARY KEY,
    ledger_sequence BIGINT NOT NULL REFERENCES ledgers(sequence) ON DELETE CASCADE,
    source_account VARCHAR(56) NOT NULL,
    fee BIGINT NOT NULL DEFAULT 0,
    operation_count INTEGER NOT NULL DEFAULT 0,
    successful BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Payments table: stores payment and path_payment operations
CREATE TABLE IF NOT EXISTS payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ledger_sequence BIGINT NOT NULL REFERENCES ledgers(sequence) ON DELETE CASCADE,
    transaction_hash VARCHAR(64) NOT NULL REFERENCES transactions(hash) ON DELETE CASCADE,
    operation_type VARCHAR(50) NOT NULL, -- 'payment' or 'path_payment_*'
    source_account VARCHAR(56) NOT NULL,
    destination VARCHAR(56) NOT NULL,
    asset_code VARCHAR(12),
    asset_issuer VARCHAR(56),
    amount VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Ingestion cursor: tracks last processed ledger for restart safety
CREATE TABLE IF NOT EXISTS ingestion_cursor (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1), -- I only need one row
    last_ledger_sequence BIGINT NOT NULL,
    cursor VARCHAR(255),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_transactions_ledger ON transactions(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_payments_ledger ON payments(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_payments_type ON payments(operation_type);
CREATE INDEX IF NOT EXISTS idx_payments_source ON payments(source_account);
CREATE INDEX IF NOT EXISTS idx_payments_destination ON payments(destination);
