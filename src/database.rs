use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::analytics::compute_anchor_metrics;
use crate::models::{
    Anchor, AnchorDetailResponse, AnchorMetricsHistory, Asset, CreateAnchorRequest,
    CreateCorridorRequest, IngestionState, PaymentRecord, SortBy,
};
use crate::models::corridor::{Corridor, CorridorMetrics, CorridorMetricsHistory};

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

    pub fn corridor_aggregates(&self) -> crate::db::aggregates::CorridorAggregates {
        crate::db::aggregates::CorridorAggregates::new(self.pool.clone())
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

    pub async fn get_anchor_by_stellar_account(
        &self,
        stellar_account: &str,
    ) -> Result<Option<Anchor>> {
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

    // Update anchor metrics from RPC ingestion
    pub async fn update_anchor_from_rpc(
        &self,
        stellar_account: &str,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        total_volume_usd: f64,
        avg_settlement_time_ms: i32,
        reliability_score: f64,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE anchors
            SET total_transactions = $1,
                successful_transactions = $2,
                failed_transactions = $3,
                total_volume_usd = $4,
                avg_settlement_time_ms = $5,
                reliability_score = $6,
                status = $7,
                updated_at = NOW()
            WHERE stellar_account = $8
            "#,
        )
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(total_volume_usd)
        .bind(avg_settlement_time_ms)
        .bind(reliability_score)
        .bind(status)
        .bind(stellar_account)
        .execute(&self.pool)
        .await?;

        Ok(())
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

    // Corridor operations
    pub async fn list_corridor_metrics(
        &self,
        limit: i64,
        offset: i64,
        sort_by: SortBy,
    ) -> Result<Vec<CorridorMetrics>> {
        let order_clause = match sort_by {
            SortBy::SuccessRate => "ORDER BY success_rate DESC, volume_usd DESC",
            SortBy::Volume => "ORDER BY volume_usd DESC, success_rate DESC",
        };

        let query = format!(
            r#"
            SELECT * FROM corridor_metrics
            {}
            LIMIT $1 OFFSET $2
            "#,
            order_clause
        );

        let corridors = sqlx::query_as::<_, CorridorMetrics>(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(corridors)
    }

    pub async fn get_corridor_metrics_by_key(&self, corridor_key: &str) -> Result<Option<CorridorMetrics>> {
        let corridor = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            SELECT * FROM corridor_metrics 
            WHERE corridor_key = $1
            ORDER BY date DESC
            LIMIT 1
            "#,
        )
        .bind(corridor_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(corridor)
    }

    pub async fn create_or_update_corridor_metrics(
        &self,
        corridor_key: &str,
        asset_a_code: &str,
        asset_a_issuer: &str,
        asset_b_code: &str,
        asset_b_issuer: &str,
        total_transactions: i64,
        successful_transactions: i64,
        failed_transactions: i64,
        volume_usd: f64,
    ) -> Result<CorridorMetrics> {
        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        let corridor = sqlx::query_as::<_, CorridorMetrics>(
            r#"
            INSERT INTO corridor_metrics (
                corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer,
                date, total_transactions, successful_transactions, failed_transactions,
                success_rate, volume_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (corridor_key, date) DO UPDATE SET
                total_transactions = EXCLUDED.total_transactions,
                successful_transactions = EXCLUDED.successful_transactions,
                failed_transactions = EXCLUDED.failed_transactions,
                success_rate = EXCLUDED.success_rate,
                volume_usd = EXCLUDED.volume_usd,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(corridor_key)
        .bind(asset_a_code)
        .bind(asset_a_issuer)
        .bind(asset_b_code)
        .bind(asset_b_issuer)
        .bind(Utc::now().date_naive())
        .bind(total_transactions)
        .bind(successful_transactions)
        .bind(failed_transactions)
        .bind(success_rate)
        .bind(volume_usd)
        .fetch_one(&self.pool)
        .await?;

        Ok(corridor)
    }
}

// Corridor operations (new)
impl Database {
    pub async fn create_corridor(&self, req: CreateCorridorRequest) -> Result<Corridor> {
        let corridor = sqlx::query_as::<_, Corridor>(
            r#"
            INSERT INTO corridors (
                name, source_asset_code, source_asset_issuer,
                dest_asset_code, dest_asset_issuer
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (source_asset_code, source_asset_issuer, dest_asset_code, dest_asset_issuer)
            DO UPDATE SET name = COALESCE(EXCLUDED.name, corridors.name)
            RETURNING *
            "#,
        )
        .bind(req.name)
        .bind(req.source_asset_code)
        .bind(req.source_asset_issuer)
        .bind(req.dest_asset_code)
        .bind(req.dest_asset_issuer)
        .fetch_one(&self.pool)
        .await?;

        Ok(corridor)
    }

    pub async fn get_corridor_by_id(&self, id: Uuid) -> Result<Option<Corridor>> {
        let corridor = sqlx::query_as::<_, Corridor>(
            r#"
            SELECT * FROM corridors WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(corridor)
    }

    pub async fn list_corridors(&self, limit: i64, offset: i64) -> Result<Vec<Corridor>> {
        let corridors = sqlx::query_as::<_, Corridor>(
            r#"
            SELECT * FROM corridors
            ORDER BY success_rate DESC, updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(corridors)
    }

    pub async fn update_corridor_metrics(
        &self,
        corridor_id: Uuid,
        metrics: CorridorMetrics,
    ) -> Result<Corridor> {
        let corridor = sqlx::query_as::<_, Corridor>(
            r#"
            UPDATE corridors
            SET total_transactions = $1,
                successful_transactions = $2,
                failed_transactions = $3,
                avg_settlement_latency_ms = $4,
                liquidity_depth_usd = $5,
                success_rate = $6
            WHERE id = $7
            RETURNING *
            "#,
        )
        .bind(metrics.total_transactions)
        .bind(metrics.successful_transactions)
        .bind(metrics.failed_transactions)
        .bind(metrics.avg_settlement_latency_ms.unwrap_or(0))
        .bind(metrics.liquidity_depth_usd)
        .bind(metrics.success_rate)
        .bind(corridor_id)
        .fetch_one(&self.pool)
        .await?;

        self.record_corridor_metrics_history(corridor_id, &metrics)
            .await?;

        Ok(corridor)
    }

    pub async fn record_corridor_metrics_history(
        &self,
        corridor_id: Uuid,
        metrics: &CorridorMetrics,
    ) -> Result<CorridorMetricsHistory> {
        let history = sqlx::query_as::<_, CorridorMetricsHistory>(
            r#"
            INSERT INTO corridor_metrics_history (
                corridor_id, timestamp, success_rate, avg_settlement_latency_ms,
                liquidity_depth_usd, total_transactions, successful_transactions,
                failed_transactions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(corridor_id)
        .bind(Utc::now())
        .bind(metrics.success_rate)
        .bind(metrics.avg_settlement_latency_ms.unwrap_or(0))
        .bind(metrics.liquidity_depth_usd)
        .bind(metrics.total_transactions)
        .bind(metrics.successful_transactions)
        .bind(metrics.failed_transactions)
        .fetch_one(&self.pool)
        .await?;

        Ok(history)
    }

    // Payment operations
    pub async fn save_payments(&self, payments: Vec<PaymentRecord>) -> Result<()> {
        if payments.is_empty() {
            return Ok(());
        }

        for payment in payments {
            sqlx::query(
                r#"
                INSERT INTO payments (
                    id, transaction_hash, source_account, destination_account,
                    asset_type, asset_code, asset_issuer, amount, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(&payment.id)
            .bind(&payment.transaction_hash)
            .bind(&payment.source_account)
            .bind(&payment.destination_account)
            .bind(&payment.asset_type)
            .bind(&payment.asset_code)
            .bind(&payment.asset_issuer)
            .bind(payment.amount)
            .bind(payment.created_at)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_ingestion_cursor(&self, task_name: &str) -> Result<Option<String>> {
        let state = sqlx::query_as::<_, IngestionState>(
            "SELECT * FROM ingestion_state WHERE task_name = $1",
        )
        .bind(task_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(state.map(|s| s.last_cursor))
    }

    pub async fn update_ingestion_cursor(&self, task_name: &str, cursor: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ingestion_state (task_name, last_cursor, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (task_name) DO UPDATE SET
                last_cursor = EXCLUDED.last_cursor,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(task_name)
        .bind(cursor)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
