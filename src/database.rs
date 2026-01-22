use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::analytics::compute_anchor_metrics;
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CreateAnchorRequest,
};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // Anchor operations
    pub async fn create_anchor(&self, req: CreateAnchorRequest) -> Result<Anchor> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            INSERT INTO anchors (name, stellar_account, home_domain)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(&req.name)
        .bind(&req.stellar_account)
        .bind(&req.home_domain)
        .fetch_one(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn get_anchor_by_stellar_account(&self, stellar_account: &str) -> Result<Option<Anchor>> {
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors WHERE stellar_account = $1
            "#,
        )
        .bind(stellar_account)
        .fetch_optional(&self.pool)
        .await?;

        Ok(anchor)
    }

    pub async fn list_anchors(&self, limit: i64, offset: i64) -> Result<Vec<Anchor>> {
        let anchors = sqlx::query_as::<_, Anchor>(
            r#"
            SELECT * FROM anchors
            ORDER BY reliability_score DESC, updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(anchors)
    }

    pub async fn update_anchor_metrics(
        &self,
        anchor_id: Uuid,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        avg_settlement_time_ms: Option<i32>,
        volume_usd: Option<f64>,
    ) -> Result<Anchor> {
        // Compute metrics
        let metrics = compute_anchor_metrics(
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
        );

        // Update anchor
        let anchor = sqlx::query_as::<_, Anchor>(
            r#"
            UPDATE anchors
            SET total_transactions = $1,
                successful_transactions = $2,
                failed_transactions = $3,
                avg_settlement_time_ms = $4,
                reliability_score = $5,
                status = $6,
                total_volume_usd = COALESCE($7, total_volume_usd)
            WHERE id = $8
            RETURNING *
            "#,
        )
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(avg_settlement_time_ms.unwrap_or(0))
        .bind(metrics.reliability_score)
        .bind(metrics.status.as_str())
        .bind(volume_usd.unwrap_or(0.0))
        .bind(anchor_id)
        .fetch_one(&self.pool)
        .await?;

        // Record metrics history
        self.record_anchor_metrics_history(
            anchor_id,
            metrics.success_rate,
            metrics.failure_rate,
            metrics.reliability_score,
            total_transactions,
            successful_transactions,
            failed_transactions,
            avg_settlement_time_ms,
            volume_usd,
        )
        .await?;

        Ok(anchor)
    }

    // Asset operations
    pub async fn create_asset(
        &self,
        anchor_id: Uuid,
        asset_code: String,
        asset_issuer: String,
    ) -> Result<Asset> {
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            INSERT INTO assets (anchor_id, asset_code, asset_issuer)
            VALUES ($1, $2, $3)
            ON CONFLICT (asset_code, asset_issuer) DO UPDATE
            SET anchor_id = EXCLUDED.anchor_id
            RETURNING *
            "#,
        )
        .bind(anchor_id)
        .bind(&asset_code)
        .bind(&asset_issuer)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    pub async fn get_assets_by_anchor(&self, anchor_id: Uuid) -> Result<Vec<Asset>> {
        let assets = sqlx::query_as::<_, Asset>(
            r#"
            SELECT * FROM assets WHERE anchor_id = $1
            ORDER BY asset_code ASC
            "#,
        )
        .bind(anchor_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(assets)
    }

    pub async fn count_assets_by_anchor(&self, anchor_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM assets WHERE anchor_id = $1
            "#,
        )
        .bind(anchor_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // Metrics history operations
    pub async fn record_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        success_rate: f64,
        failure_rate: f64,
        reliability_score: f64,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        avg_settlement_time_ms: Option<i32>,
        volume_usd: Option<f64>,
    ) -> Result<AnchorMetricsHistory> {
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r#"
            INSERT INTO anchor_metrics_history (
                anchor_id, timestamp, success_rate, failure_rate, reliability_score,
                total_transactions, successful_transactions, failed_transactions,
                avg_settlement_time_ms, volume_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(anchor_id)
        .bind(Utc::now())
        .bind(success_rate)
        .bind(failure_rate)
        .bind(reliability_score)
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(avg_settlement_time_ms.unwrap_or(0))
        .bind(volume_usd.unwrap_or(0.0))
        .fetch_one(&self.pool)
        .await?;

        Ok(history)
    }

    pub async fn get_anchor_metrics_history(
        &self,
        anchor_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AnchorMetricsHistory>> {
        let history = sqlx::query_as::<_, AnchorMetricsHistory>(
            r#"
            SELECT * FROM anchor_metrics_history
            WHERE anchor_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(anchor_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }

    pub async fn get_anchor_detail(&self, anchor_id: Uuid) -> Result<Option<AnchorDetailResponse>> {
        let anchor = match self.get_anchor_by_id(anchor_id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        let assets = self.get_assets_by_anchor(anchor_id).await?;
        let metrics_history = self.get_anchor_metrics_history(anchor_id, 30).await?;

        Ok(Some(AnchorDetailResponse {
            anchor,
            assets,
            metrics_history,
        }))
    }
}
