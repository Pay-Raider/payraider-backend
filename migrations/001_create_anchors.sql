-- Create anchors table for tracking anchor performance metrics
CREATE TABLE IF NOT EXISTS anchors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    stellar_account VARCHAR(56) NOT NULL UNIQUE,
    home_domain VARCHAR(255),
    total_transactions BIGINT DEFAULT 0,
    successful_transactions BIGINT DEFAULT 0,
    failed_transactions BIGINT DEFAULT 0,
    total_volume_usd DECIMAL(20, 2) DEFAULT 0,
    avg_settlement_time_ms INTEGER DEFAULT 0,
    reliability_score DECIMAL(5, 2) DEFAULT 0,
    status VARCHAR(20) DEFAULT 'green',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create assets table for tracking issued assets per anchor
CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    anchor_id UUID NOT NULL REFERENCES anchors(id) ON DELETE CASCADE,
    asset_code VARCHAR(12) NOT NULL,
    asset_issuer VARCHAR(56) NOT NULL,
    total_supply DECIMAL(30, 7),
    num_holders BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(asset_code, asset_issuer)
);

-- Create anchor_metrics_history table for time-series data
CREATE TABLE IF NOT EXISTS anchor_metrics_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    anchor_id UUID NOT NULL REFERENCES anchors(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    success_rate DECIMAL(5, 2) NOT NULL,
    failure_rate DECIMAL(5, 2) NOT NULL,
    reliability_score DECIMAL(5, 2) NOT NULL,
    total_transactions BIGINT NOT NULL,
    successful_transactions BIGINT NOT NULL,
    failed_transactions BIGINT NOT NULL,
    avg_settlement_time_ms INTEGER,
    volume_usd DECIMAL(20, 2),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX idx_anchors_reliability ON anchors(reliability_score DESC);
CREATE INDEX idx_anchors_status ON anchors(status);
CREATE INDEX idx_anchors_stellar_account ON anchors(stellar_account);
CREATE INDEX idx_assets_anchor ON assets(anchor_id);
CREATE INDEX idx_anchor_metrics_anchor_time ON anchor_metrics_history(anchor_id, timestamp DESC);
CREATE INDEX idx_anchor_metrics_timestamp ON anchor_metrics_history(timestamp DESC);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_anchors_updated_at BEFORE UPDATE ON anchors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_assets_updated_at BEFORE UPDATE ON assets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
