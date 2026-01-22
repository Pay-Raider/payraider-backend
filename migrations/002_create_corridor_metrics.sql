-- Create corridor_metrics table for tracking payment corridor performance
CREATE TABLE IF NOT EXISTS corridor_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    corridor_key VARCHAR(255) NOT NULL,
    asset_a_code VARCHAR(12) NOT NULL,
    asset_a_issuer VARCHAR(56) NOT NULL,
    asset_b_code VARCHAR(12) NOT NULL,
    asset_b_issuer VARCHAR(56) NOT NULL,
    date DATE NOT NULL,
    total_transactions BIGINT DEFAULT 0,
    successful_transactions BIGINT DEFAULT 0,
    failed_transactions BIGINT DEFAULT 0,
    success_rate DECIMAL(5, 2) DEFAULT 0,
    volume_usd DECIMAL(20, 2) DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(corridor_key, date)
);

-- Create indexes for better query performance
CREATE INDEX idx_corridor_metrics_key ON corridor_metrics(corridor_key);
CREATE INDEX idx_corridor_metrics_success_rate ON corridor_metrics(success_rate DESC);
CREATE INDEX idx_corridor_metrics_volume ON corridor_metrics(volume_usd DESC);
CREATE INDEX idx_corridor_metrics_date ON corridor_metrics(date DESC);
CREATE INDEX idx_corridor_metrics_assets ON corridor_metrics(asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer);

-- Create trigger for updated_at
CREATE TRIGGER update_corridor_metrics_updated_at BEFORE UPDATE ON corridor_metrics
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();