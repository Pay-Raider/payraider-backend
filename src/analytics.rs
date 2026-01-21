use crate::models::{AnchorMetrics, AnchorStatus};

/// Compute anchor reliability metrics based on transaction data
pub fn compute_anchor_metrics(
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    avg_settlement_time_ms: Option<i32>,
) -> AnchorMetrics {
    if total_transactions == 0 {
        return AnchorMetrics {
            success_rate: 0.0,
            failure_rate: 0.0,
            reliability_score: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            avg_settlement_time_ms: None,
            status: AnchorStatus::Red,
        };
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let failure_rate = (failed_transactions as f64 / total_transactions as f64) * 100.0;

    // Compute reliability score (0-100)
    // Formula: (success_rate * 0.5) + (settlement_time_score * 0.25) + (volume_consistency * 0.25)
    // For MVP, we'll use a simplified formula focused on success rate and settlement time
    let settlement_time_score = calculate_settlement_time_score(avg_settlement_time_ms);
    let reliability_score = (success_rate * 0.7) + (settlement_time_score * 0.3);

    let status = AnchorStatus::from_metrics(success_rate, failure_rate);

    AnchorMetrics {
        success_rate,
        failure_rate,
        reliability_score,
        total_transactions,
        successful_transactions,
        failed_transactions,
        avg_settlement_time_ms,
        status,
    }
}

/// Calculate settlement time score (0-100)
/// Lower settlement time = higher score
fn calculate_settlement_time_score(avg_settlement_time_ms: Option<i32>) -> f64 {
    const MAX_SETTLEMENT_TIME_MS: f64 = 10000.0; // 10 seconds
    const MIN_SETTLEMENT_TIME_MS: f64 = 1000.0; // 1 second

    match avg_settlement_time_ms {
        Some(time_ms) if time_ms <= MIN_SETTLEMENT_TIME_MS as i32 => 100.0,
        Some(time_ms) if time_ms >= MAX_SETTLEMENT_TIME_MS as i32 => 0.0,
        Some(time_ms) => {
            let normalized = (MAX_SETTLEMENT_TIME_MS - time_ms as f64)
                / (MAX_SETTLEMENT_TIME_MS - MIN_SETTLEMENT_TIME_MS);
            normalized * 100.0
        }
        None => 50.0, // Default middle score if no data
    }
}

/// Calculate assets issued per anchor
pub fn count_assets_per_anchor(assets: &[String]) -> usize {
    assets.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_anchor_metrics_perfect_anchor() {
        let metrics = compute_anchor_metrics(1000, 990, 10, Some(2000));

        assert_eq!(metrics.total_transactions, 1000);
        assert_eq!(metrics.successful_transactions, 990);
        assert_eq!(metrics.failed_transactions, 10);
        assert_eq!(metrics.success_rate, 99.0);
        assert_eq!(metrics.failure_rate, 1.0);
        assert!(metrics.reliability_score > 90.0);
        assert_eq!(metrics.status, AnchorStatus::Green);
    }

    #[test]
    fn test_compute_anchor_metrics_yellow_anchor() {
        let metrics = compute_anchor_metrics(1000, 960, 40, Some(5000));

        assert_eq!(metrics.success_rate, 96.0);
        assert_eq!(metrics.failure_rate, 4.0);
        assert_eq!(metrics.status, AnchorStatus::Yellow);
    }

    #[test]
    fn test_compute_anchor_metrics_red_anchor() {
        let metrics = compute_anchor_metrics(1000, 900, 100, Some(9000));

        assert_eq!(metrics.success_rate, 90.0);
        assert_eq!(metrics.failure_rate, 10.0);
        assert_eq!(metrics.status, AnchorStatus::Red);
    }

    #[test]
    fn test_compute_anchor_metrics_no_transactions() {
        let metrics = compute_anchor_metrics(0, 0, 0, None);

        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.failure_rate, 0.0);
        assert_eq!(metrics.reliability_score, 0.0);
        assert_eq!(metrics.status, AnchorStatus::Red);
    }

    #[test]
    fn test_settlement_time_score_fast() {
        let score = calculate_settlement_time_score(Some(500));
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_settlement_time_score_slow() {
        let score = calculate_settlement_time_score(Some(12000));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_settlement_time_score_medium() {
        let score = calculate_settlement_time_score(Some(5000));
        assert!(score > 40.0 && score < 60.0);
    }

    #[test]
    fn test_count_assets() {
        let assets = vec!["USDC".to_string(), "EURC".to_string(), "BTC".to_string()];
        assert_eq!(count_assets_per_anchor(&assets), 3);
    }
}
