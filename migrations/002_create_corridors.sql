-- Corridors tables for health metrics per asset pair
CREATE TABLE IF NOT EXISTS corridors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255),
    source_asset_code VARCHAR(12) NOT NULL,
    source_asset_issuer VARCHAR(56) NOT NULL,
    dest_asset_code VARCHAR(12) NOT NULL,
    dest_asset_issuer VARCHAR(56) NOT NULL,
    total_transactions BIGINT DEFAULT 0,
    successful_transactions BIGINT DEFAULT 0,
    failed_transactions BIGINT DEFAULT 0,
    avg_settlement_latency_ms INTEGER DEFAULT 0,
    liquidity_depth_usd DECIMAL(20, 2) DEFAULT 0,
    success_rate DECIMAL(5, 2) DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (source_asset_code, source_asset_issuer, dest_asset_code, dest_asset_issuer)
);

CREATE TABLE IF NOT EXISTS corridor_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    corridor_id UUID NOT NULL REFERENCES corridors(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    success_rate DECIMAL(5, 2) NOT NULL,
    avg_settlement_latency_ms INTEGER,
    liquidity_depth_usd DECIMAL(20, 2) NOT NULL,
    total_transactions BIGINT NOT NULL,
    successful_transactions BIGINT NOT NULL,
    failed_transactions BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_corridors_success_rate ON corridors(success_rate DESC);
CREATE INDEX IF NOT EXISTS idx_corridors_assets ON corridors(source_asset_code, dest_asset_code);
CREATE INDEX IF NOT EXISTS idx_corridor_metrics_corridor_time ON corridor_metrics_history(corridor_id, timestamp DESC);

-- Triggers for updated_at
CREATE TRIGGER update_corridors_updated_at BEFORE UPDATE ON corridors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
