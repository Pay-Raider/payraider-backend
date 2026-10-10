//! Rolling window of recent mainnet payments.
//!
//! Horizon's payments feed moves fast: a single 1,000-record fetch covers
//! well under a minute of mainnet activity, so corridors scored from it rest
//! on a handful of payments. This keeps every payment seen over the last
//! `PAYMENT_WINDOW_HOURS` (default 24) in memory, topping it up from the head
//! of the feed on a timer, and scores corridors from that instead.
//!
//! Self-payments (same account on both ends, mostly arbitrage bots cycling
//! path payments through the DEX) are dropped on the way in: they say nothing
//! about whether a payment to someone else would settle.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::rpc::{Payment, StellarRpcClient};

const PAGE_SIZE: u32 = 200;
const DEFAULT_WINDOW_HOURS: i64 = 24;
const DEFAULT_MAX_PAYMENTS: usize = 50_000;
const DEFAULT_REFRESH_SECS: u64 = 60;
/// Pages read per refresh. Bounds Horizon load; if the feed outruns it, the
/// window has a gap rather than falling further and further behind.
const MAX_PAGES_PER_REFRESH: usize = 25;

/// What the window currently covers, for logs and API responses.
#[derive(Debug, Clone, Copy, serde::Serialize, utoipa::ToSchema)]
pub struct WindowCoverage {
    pub payments: usize,
    pub oldest: Option<DateTime<Utc>>,
    pub newest: Option<DateTime<Utc>>,
}

#[derive(Default)]
struct State {
    /// Newest first, matching Horizon's `order=desc`.
    payments: VecDeque<Payment>,
    ids: HashSet<String>,
}

pub struct PaymentWindow {
    state: RwLock<State>,
    window: chrono::Duration,
    max_payments: usize,
}

/// Same account sends and receives: arbitrage cycles and wallet shuffles.
#[must_use]
pub fn is_self_payment(payment: &Payment) -> bool {
    let from = payment.from.as_deref().unwrap_or(&payment.source_account);
    let to = payment.to.as_deref().unwrap_or(&payment.destination);
    !to.is_empty() && from == to
}

fn created_at(payment: &Payment) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&payment.created_at)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

impl PaymentWindow {
    #[must_use]
    pub fn new(window: chrono::Duration, max_payments: usize) -> Self {
        Self {
            state: RwLock::new(State::default()),
            window,
            max_payments,
        }
    }

    fn from_env() -> Self {
        Self::new(
            chrono::Duration::hours(env_or("PAYMENT_WINDOW_HOURS", DEFAULT_WINDOW_HOURS)),
            env_or("PAYMENT_WINDOW_MAX", DEFAULT_MAX_PAYMENTS),
        )
    }

    /// The process-wide window shared by the refresher and the API.
    pub fn global() -> &'static PaymentWindow {
        static WINDOW: OnceLock<PaymentWindow> = OnceLock::new();
        WINDOW.get_or_init(Self::from_env)
    }

    /// Payments in the window, newest first. Empty until the first refresh.
    pub async fn snapshot(&self) -> Vec<Payment> {
        self.state.read().await.payments.iter().cloned().collect()
    }

    pub async fn coverage(&self) -> WindowCoverage {
        let state = self.state.read().await;
        WindowCoverage {
            payments: state.payments.len(),
            newest: state.payments.front().and_then(created_at),
            oldest: state.payments.back().and_then(created_at),
        }
    }

    /// Add a newest-first batch of payments fetched from the head of the feed.
    /// Returns how many were new.
    pub async fn ingest(&self, batch: Vec<Payment>, now: DateTime<Utc>) -> usize {
        let mut state = self.state.write().await;
        let fresh: Vec<Payment> = batch
            .into_iter()
            .filter(|p| !state.ids.contains(&p.id) && !is_self_payment(p))
            .collect();
        let added = fresh.len();

        // `fresh` is newest first; push in reverse so the newest ends up at
        // the front.
        for payment in fresh.into_iter().rev() {
            state.ids.insert(payment.id.clone());
            state.payments.push_front(payment);
        }

        let cutoff = now - self.window;
        while let Some(oldest) = state.payments.back() {
            let expired = created_at(oldest).is_none_or(|t| t < cutoff);
            if !expired && state.payments.len() <= self.max_payments {
                break;
            }
            if let Some(p) = state.payments.pop_back() {
                state.ids.remove(&p.id);
            }
        }
        added
    }

    async fn knows(&self, id: &str) -> bool {
        self.state.read().await.ids.contains(id)
    }

    /// Read from the head of the feed until reaching payments already held.
    pub async fn refresh(&self, rpc: &StellarRpcClient) -> anyhow::Result<usize> {
        let mut batch = Vec::new();
        let mut cursor: Option<String> = None;
        let mut caught_up = false;

        for _ in 0..MAX_PAGES_PER_REFRESH {
            let page = rpc.fetch_payments(PAGE_SIZE, cursor.as_deref()).await?;
            let Some(last) = page.last() else { break };
            cursor = Some(last.paging_token.clone());

            let mut seen_known = false;
            for payment in page {
                if self.knows(&payment.id).await {
                    seen_known = true;
                    break;
                }
                batch.push(payment);
            }
            if seen_known {
                caught_up = true;
                break;
            }
        }

        let added = self.ingest(batch, Utc::now()).await;
        let has_history = self.coverage().await.payments > added;
        if !caught_up && has_history {
            warn!(
                "Payment window fell behind the feed; read {} pages without reaching held payments",
                MAX_PAGES_PER_REFRESH
            );
        }
        Ok(added)
    }

    /// Keep the window topped up until the process exits.
    pub fn spawn_refresher(rpc: Arc<StellarRpcClient>) -> tokio::task::JoinHandle<()> {
        let every =
            Duration::from_secs(env_or("PAYMENT_WINDOW_REFRESH_SECS", DEFAULT_REFRESH_SECS));
        tokio::spawn(async move {
            let window = Self::global();
            let mut interval = tokio::time::interval(every);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                match window.refresh(&rpc).await {
                    Ok(added) => {
                        let c = window.coverage().await;
                        info!(
                            added,
                            held = c.payments,
                            oldest = ?c.oldest,
                            "Payment window refreshed"
                        );
                    }
                    Err(e) => warn!("Payment window refresh failed: {e}"),
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payment(id: &str, from: &str, to: &str, at: &str) -> Payment {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "paging_token": id,
            "transaction_hash": "h",
            "source_account": from,
            "from": from,
            "to": to,
            "asset_type": "native",
            "amount": "1",
            "created_at": at,
        }))
        .unwrap()
    }

    fn now() -> DateTime<Utc> {
        "2026-10-10T12:00:00Z".parse().unwrap()
    }

    #[tokio::test]
    async fn drops_self_payments_and_duplicates() {
        let w = PaymentWindow::new(chrono::Duration::hours(24), 100);
        let added = w
            .ingest(
                vec![
                    payment("2", "A", "B", "2026-10-10T11:59:00Z"),
                    payment("1", "C", "C", "2026-10-10T11:58:00Z"),
                ],
                now(),
            )
            .await;
        assert_eq!(added, 1);
        let again = w
            .ingest(vec![payment("2", "A", "B", "2026-10-10T11:59:00Z")], now())
            .await;
        assert_eq!(again, 0);
    }

    #[tokio::test]
    async fn keeps_newest_first_and_prunes_old() {
        let w = PaymentWindow::new(chrono::Duration::hours(1), 100);
        w.ingest(vec![payment("1", "A", "B", "2026-10-10T10:00:00Z")], now())
            .await;
        w.ingest(
            vec![
                payment("3", "A", "B", "2026-10-10T11:59:00Z"),
                payment("2", "A", "B", "2026-10-10T11:30:00Z"),
            ],
            now(),
        )
        .await;
        let ids: Vec<String> = w.snapshot().await.into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["3", "2"]);
    }

    #[tokio::test]
    async fn caps_size() {
        let w = PaymentWindow::new(chrono::Duration::hours(24), 2);
        w.ingest(
            vec![
                payment("3", "A", "B", "2026-10-10T11:59:00Z"),
                payment("2", "A", "B", "2026-10-10T11:58:00Z"),
                payment("1", "A", "B", "2026-10-10T11:57:00Z"),
            ],
            now(),
        )
        .await;
        assert_eq!(w.coverage().await.payments, 2);
    }
}
