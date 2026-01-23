use crate::models::corridor::CorridorMetrics;

#[derive(Debug, Clone)]
pub struct CorridorTransaction {
    pub successful: bool,
    pub settlement_latency_ms: Option<i32>,
    pub amount_usd: f64,
}

/// Compute corridor metrics from a set of transactions
pub fn compute_corridor_metrics(txns: &[CorridorTransaction]) -> CorridorMetrics {
    if txns.is_empty() {
        return CorridorMetrics {
            id: uuid::Uuid::nil(),
            corridor_key: String::new(),
            asset_a_code: String::new(),
            asset_a_issuer: String::new(),
            asset_b_code: String::new(),
            asset_b_issuer: String::new(),
            date: chrono::Utc::now(),
            success_rate: 0.0,
            avg_settlement_latency_ms: None,
            liquidity_depth_usd: 0.0,
            volume_usd: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
    }

    let total_transactions = txns.len() as i64;

    let mut successful_transactions: i64 = 0;
    let mut failed_transactions: i64 = 0;
    let mut latency_sum: i64 = 0;
    let mut latency_count: i64 = 0;
    let mut volume_usd: f64 = 0.0;

    for t in txns {
        if t.successful {
            successful_transactions += 1;
            volume_usd += t.amount_usd.max(0.0);
            if let Some(ms) = t.settlement_latency_ms {
                latency_sum += ms as i64;
                latency_count += 1;
            }
        } else {
            failed_transactions += 1;
        }
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let avg_settlement_latency_ms = if latency_count > 0 {
        Some((latency_sum / latency_count) as i32)
    } else {
        None
    };

    CorridorMetrics {
        id: uuid::Uuid::nil(),
        corridor_key: String::new(),
        asset_a_code: String::new(),
        asset_a_issuer: String::new(),
        asset_b_code: String::new(),
        asset_b_issuer: String::new(),
        date: chrono::Utc::now(),
        total_transactions,
        successful_transactions,
        failed_transactions,
        success_rate,
        volume_usd,
        avg_settlement_latency_ms,
        liquidity_depth_usd: 0.0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_corridor_metrics_basic() {
        let txns = vec![
            CorridorTransaction { successful: true, settlement_latency_ms: Some(1000), amount_usd: 100.0 },
            CorridorTransaction { successful: true, settlement_latency_ms: Some(3000), amount_usd: 200.0 },
            CorridorTransaction { successful: false, settlement_latency_ms: None, amount_usd: 50.0 },
        ];

        let metrics = compute_corridor_metrics(&txns);
        assert_eq!(metrics.total_transactions, 3);
        assert_eq!(metrics.successful_transactions, 2);
        assert_eq!(metrics.failed_transactions, 1);
        assert_eq!(metrics.success_rate, (2.0/3.0)*100.0);
        assert_eq!(metrics.avg_settlement_latency_ms, Some(2000));
        assert_eq!(metrics.liquidity_depth_usd, 300.0);
    }

    #[test]
    fn test_compute_corridor_metrics_empty() {
        let metrics = compute_corridor_metrics(&[]);
        assert_eq!(metrics.total_transactions, 0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.avg_settlement_latency_ms, None);
        assert_eq!(metrics.liquidity_depth_usd, 0.0);
    }

    #[test]
    fn test_compute_corridor_metrics_all_failed() {
        let txns = vec![
            CorridorTransaction { successful: false, settlement_latency_ms: None, amount_usd: 10.0 },
            CorridorTransaction { successful: false, settlement_latency_ms: None, amount_usd: 20.0 },
        ];
        let metrics = compute_corridor_metrics(&txns);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.avg_settlement_latency_ms, None);
        assert_eq!(metrics.liquidity_depth_usd, 0.0);
    }
}
