use crate::models::corridor::CorridorMetrics;

#[derive(Debug, Clone)]
pub struct CorridorTransaction {
    pub successful: bool,
    pub settlement_latency_ms: Option<i32>,
    pub amount_usd: f64,
}

/// Order book structures for computing liquidity depth
#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub price: f64,
    pub amount_usd: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub bids: Vec<OrderBookEntry>, // Descending by price
    pub asks: Vec<OrderBookEntry>, // Ascending by price
}

/// Compute total liquidity in USD within a max slippage percent
pub fn compute_liquidity_depth(order_book: &OrderBookSnapshot, max_slippage_percent: f64) -> f64 {
    if order_book.bids.is_empty() && order_book.asks.is_empty() {
        return 0.0;
    }

    let best_bid = order_book.bids.first().map(|b| b.price).unwrap_or(0.0);
    let best_ask = order_book.asks.first().map(|a| a.price).unwrap_or(0.0);
    if best_bid == 0.0 || best_ask == 0.0 {
        return 0.0;
    }

    let mid_price = (best_bid + best_ask) / 2.0;
    let max_buy_price = mid_price * (1.0 + max_slippage_percent / 100.0);
    let min_sell_price = mid_price * (1.0 - max_slippage_percent / 100.0);

    // Buy-side liquidity (asks within max slippage)
    let buy_liquidity: f64 = order_book.asks
        .iter()
        .take_while(|a| a.price <= max_buy_price)
        .map(|a| a.amount_usd)
        .sum();

    // Sell-side liquidity (bids within max slippage)
    let sell_liquidity: f64 = order_book.bids
        .iter()
        .take_while(|b| b.price >= min_sell_price)
        .map(|b| b.amount_usd)
        .sum();

    buy_liquidity + sell_liquidity
}

/// Compute corridor metrics from transactions + optional order book
pub fn compute_corridor_metrics(
    txns: &[CorridorTransaction],
    order_book: Option<&OrderBookSnapshot>, // Optional snapshot for liquidity depth
    slippage_percent: f64,                  // e.g., 1.0 = 1% slippage
) -> CorridorMetrics {
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
    let mut successful_transactions = 0;
    let mut failed_transactions = 0;
    let mut latency_sum = 0;
    let mut latency_count = 0;
    let mut volume_usd = 0.0;

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

    // Compute liquidity depth using order book snapshot if provided
    let liquidity_depth_usd = order_book
        .map(|ob| compute_liquidity_depth(ob, slippage_percent))
        .unwrap_or(0.0);

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
        liquidity_depth_usd,
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

        let order_book = OrderBookSnapshot {
            bids: vec![
                OrderBookEntry { price: 99.0, amount_usd: 150.0 },
                OrderBookEntry { price: 98.5, amount_usd: 100.0 },
            ],
            asks: vec![
                OrderBookEntry { price: 101.0, amount_usd: 200.0 },
                OrderBookEntry { price: 102.0, amount_usd: 50.0 },
            ],
        };

        let metrics = compute_corridor_metrics(&txns, Some(&order_book), 1.0); // 1% slippage
        assert_eq!(metrics.total_transactions, 3);
        assert_eq!(metrics.successful_transactions, 2);
        assert_eq!(metrics.failed_transactions, 1);
        assert_eq!(metrics.success_rate, (2.0/3.0)*100.0);
        assert_eq!(metrics.avg_settlement_latency_ms, Some(2000));
        assert!(metrics.liquidity_depth_usd > 0.0); // computed from order book
    }

    #[test]
    fn test_compute_corridor_metrics_empty() {
        let metrics = compute_corridor_metrics(&[], None, 1.0);
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
        let metrics = compute_corridor_metrics(&txns, None, 1.0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.avg_settlement_latency_ms, None);
        assert_eq!(metrics.liquidity_depth_usd, 0.0);
    }
}
