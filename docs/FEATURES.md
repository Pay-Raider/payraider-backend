# Ndii Intelligence Dashboard - Features & Use Cases

## 📊 Core Features

### 1. Payment Reliability Analytics

**What It Measures**
- Success rate of payments across different corridors (asset pair routes)
- Failed payment patterns and root causes
- Payment confirmation latency
- Correlation between payment success and network conditions

**Who Uses It**
- Wallet developers predict payment success before sending
- Remittance services identify reliable routes
- DeFi protocols optimize transaction routing

**Business Value**
- Reduce failed transactions
- Improve user experience with reliable payments
- Lower operational costs from failed retries

---

### 2. Corridor Health Scoring

**What It Measures**
- **Liquidity Depth**: Order book capacity for asset pairs
- **Bid-Ask Spread**: Price efficiency of trading pairs
- **Volume Trends**: 24h, 7d, 30d payment volumes
- **Slippage**: Price impact of executing trades
- **Success Rate**: % of successful trades vs attempts

**Dashboard Display**
- Heatmap showing corridor reliability at a glance
- Color coding: 🟢 Green (reliable) → 🟡 Yellow (caution) → 🔴 Red (fragile)
- Trend indicators showing improvement/degradation

**Business Value**
- Identify optimal payment routes
- Spot liquidity bottlenecks before they impact operations
- Plan asset provisioning based on real demand data

---

### 3. Anchor/Issuer Performance Tracking

**What It Measures**
- Asset issuers (anchors) reliability scores
- Transaction success rates per anchor
- Failure frequency and refund rates
- Asset portfolio health
- Time-series performance trends

**Status Indicators**
- 🟢 **Green**: Highly reliable (>98% success, <1% failures)
- 🟡 **Yellow**: Caution zone (95-98% success, 1-5% failures)
- 🔴 **Red**: At risk (<95% success, >5% failures)

**Business Value**
- Assess counterparty risk before accepting payments
- Optimize settlement with reliable anchors
- Identify anchors needing liquidity support

---

### 4. Liquidity Dynamics Monitoring

**What It Measures**
- Total Value Locked (TVL) across all corridors
- Active liquidity availability
- Peak vs off-peak liquidity patterns
- Liquidity stress scenarios
- Temporal trends (daily, weekly, monthly)

**Visualizations**
- Time-series charts of liquidity changes
- 24-hour heatmap of liquidity availability
- Volume distribution across assets

**Business Value**
- Plan payment scheduling during high-liquidity periods
- Predict payment timing impact
- Allocate assets to highest-demand corridors

---

### 5. Network Analytics Dashboard

**KPI Cards**
- **Payment Success Rate**: Real-time network success percentage
- **Active Corridors**: Total number of active payment routes
- **Liquidity Depth**: Total TVL in major corridors
- **Average Settlement Time**: Median payment confirmation time

**Charts**
- Payment volume trends over time
- Success rate by corridor
- Anchor performance rankings
- Geographic distribution (if applicable)

**Business Value**
- Quick network health check
- Identify network-wide issues immediately
- Spot anomalies and trends

---

## 🎯 Use Case Scenarios

### Scenario 1: Remittance Service Selecting Routes

**User**: Remittance app sending USD → BRL payment

**Current Challenge**: 
- Which corridor has best success + lowest slippage?
- Is USDC → BRL more reliable than XLM → BRL?

**How Ndii Helps**:
1. Open Corridors page
2. Compare USDC→BRL vs XLM→BRL metrics
3. View slippage, success rate, volume trends
4. Choose optimal route with confidence

**Outcome**: ✅ Payment succeeds at <1% slippage

---

### Scenario 2: Wallet Predicting Payment Success

**User**: Mobile wallet user sending payment

**Current Challenge**:
- Will this payment succeed?
- Should I retry if it fails?
- What's the expected settlement time?

**How Ndii Helps**:
1. Show predicted success % for this corridor
2. Display current liquidity status
3. Recommend alternative routes if primary fails
4. Show expected settlement time

**Outcome**: ✅ 99.2% payment success rate via suggested route

---

### Scenario 3: Anchor Assessing Network Health

**User**: Asset issuer (e.g., Circle for USDC)

**Current Challenge**:
- How is my asset performing?
- Which corridors drive adoption?
- Should I increase liquidity provisioning?

**How Ndii Helps**:
1. View anchor performance scorecard
2. See USDC success rates across corridors
3. Identify USDC→XLM as high-volume but low-slippage corridor
4. Allocate more liquidity there

**Outcome**: ✅ Improved network efficiency, more volume

---

### Scenario 4: DeFi Protocol Optimizing Routing

**User**: Stellar DEX or aggregator protocol

**Current Challenge**:
- What's the best multi-hop route for swaps?
- Which corridors are least congested?
- How do route changes affect user experience?

**How Ndii Helps**:
1. Query API for best routes across all corridors
2. Get slippage estimates for each hop
3. Route based on real liquidity + success rates
4. Monitor and auto-rebalance routes

**Outcome**: ✅ Better user swaps, lower slippage

---

### Scenario 5: Compliance Team Monitoring Risk

**User**: Compliance officer, payments startup

**Current Challenge**:
- Are there fragile corridors creating systemic risk?
- Which anchors are failing most often?
- Should we halt payments on certain routes?

**How Ndii Helps**:
1. View network-wide reliability heatmap
2. Identify red-flagged corridors
3. Track anchor failure rates
4. Set alerts for threshold breaches

**Outcome**: ✅ Early risk detection, proactive mitigation

---

## 📈 Key Metrics Reference

### Payment Success Rate
**Formula**: (Successful Payments / Total Attempted Payments) × 100

**Thresholds**:
- ≥99%: Excellent
- 95-99%: Good
- 90-95%: Acceptable
- <90%: Poor (needs investigation)

### Corridor Slippage
**Definition**: Price difference between expected and executed price

**Formula**: ((Executed Price - Expected Price) / Expected Price) × 100

**Benchmarks**:
- <0.1%: Excellent liquidity
- 0.1-0.5%: Good
- 0.5-2%: Moderate
- >2%: Poor liquidity

### Liquidity Depth Score
**Definition**: Order book capacity at various price levels

**Measured in**: USD value of orders within 1% of mid-price

**Benchmarks**:
- >$10M: Deep liquidity
- $1M-$10M: Moderate
- $100K-$1M: Shallow
- <$100K: Very shallow

### Anchor Reliability Score
**Components** (weighted):
- Transaction Success Rate (50%)
- Average Settlement Time (25%)
- Refund/Reversal Rate (15%)
- Asset Diversity (10%)

**Range**: 0-100
- 90+: Tier-1 reliability
- 80-89: Tier-2 reliability
- 70-79: Tier-3 reliability
- <70: At risk

---

## ✅ On-Chain Verification

Unlike typical dashboards, Ndii provides **verifiable analytics**:

### How Verification Works
1. **Backend computes** analytics from Stellar data
2. **Backend hashes** the snapshot (SHA-256)
3. **Smart contract** stores hash + timestamp on-chain
4. **Anyone can verify** off-chain data against on-chain proof
5. **Audit trail** preserved forever on Stellar ledger

### Verification Command
```bash
curl https://api.ndii.io/api/verify \
  -d '{
    "snapshot_epoch": 1234,
    "expected_hash": "abc123..."
  }'

# Response:
{
  "valid": true,
  "on_chain_hash": "abc123...",
  "timestamp": "2025-01-19T14:30:00Z",
  "contract_id": "CAE3HQ...",
  "stellar_proof": "..."
}
```

### Who Benefits from Verification
- **Regulatory bodies** – Audit analytics independently
- **Anchor compliance teams** – Prove data integrity
- **Institutional users** – Verify before making decisions
- **Research & academia** – Trust data sources

---

## 🔄 Data Refresh & Timing

| Metric | Computation | On-Chain Anchor | Data Window |
|--------|------------|------------------|-------------|
| Real-time KPIs | Every 1 minute | N/A | Current state |
| Payment Success Rate | Every 5 minutes | Hourly | Last 24 hours |
| Corridor Metrics | Every 15 minutes | Every 4 hours | Last 7 days |
| Anchor Scores | Every 30 minutes | Hourly | Last 30 days |
| Liquidity Trends | Every 30 minutes | Twice daily | Last 90 days |
| Historical Analytics | Daily | Daily | Last 12 months |
| Full Snapshot Hash | Every epoch | ✅ On-chain | Immutable proof |

---

## 🚀 Future Feature Roadmap

### Phase 2: Backend & Smart Contract
- [ ] Rust analytics engine deployment
- [ ] Soroban contract mainnet launch
- [ ] Real-time data ingestion pipeline
- [ ] On-chain snapshot anchoring
- [ ] Verification UI in dashboard

### Phase 3: Predictive Analytics
- [ ] ML-based payment success prediction
- [ ] Anomaly detection for fraud/risk
- [ ] Liquidity forecasting
- [ ] Route optimization AI engine
- [ ] Supply-demand modeling

### Phase 4: Alerting & Automation
- [ ] Real-time alerts for threshold breaches
- [ ] Custom alert rules (corridor, anchor, metric)
- [ ] Webhook notifications & integrations
- [ ] Automated routing failover recommendations
- [ ] Slack/Discord/Email notifications

### Phase 5: Advanced Reporting & Export
- [ ] Custom report generation (PDF, Excel)
- [ ] Data export (CSV, JSON, Parquet)
- [ ] Historical comparisons & trends
- [ ] Benchmarking against network averages
- [ ] SLA monitoring & uptime tracking

### Phase 6: Ecosystem Integration
- [ ] Public REST API (v1)
- [ ] Real-time WebSocket feeds
- [ ] Third-party analytics plugins
- [ ] Open data lake access (BigQuery, S3)
- [ ] GraphQL API option
- [ ] SDK for wallets & apps

### Phase 7: Advanced On-Chain Features
- [ ] Decentralized oracle for settlement times
- [ ] Liquidity pool recommendations contract
- [ ] Automated market maker (AMM) integration
- [ ] Cross-chain analytics (if Stellar bridges)
- [ ] NFT certificates for top performers

---

## 📞 Questions & Support

For feature requests or clarifications, please:
1. Check [GitHub Issues](https://github.com/Ndifreke000/stellar-insights/issues)
2. Review [Architecture.md](./ARCHITECTURE.md) for technical details
3. Open a new issue with details
