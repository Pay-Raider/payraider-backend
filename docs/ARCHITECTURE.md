# Ndii Intelligence Dashboard - Architecture & Data Design

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Interface Layer                         │
│  React SPA | TypeScript | Vite | Tailwind CSS | shadcn-ui      │
│                                                                 │
│  Landing Page → Dashboard → Corridors → Anchors → Analytics    │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    State Management Layer                       │
│  TanStack React Query | React Router | Context API              │
│                                                                 │
│  - Data caching and synchronization                            │
│  - Route state management                                       │
│  - Global notifications (Sonner/Toaster)                        │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API Layer (Future)                         │
│  REST Endpoints | WebSocket (real-time) | GraphQL (optional)   │
│                                                                 │
│  GET /api/metrics/kpi                                          │
│  GET /api/corridors                                            │
│  GET /api/anchors                                              │
│  GET /api/payments/trends                                      │
│  GET /api/liquidity                                            │
│  POST /api/predictions                                         │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Business Logic / Metrics Engine              │
│  Payment Success Scoring | Corridor Analysis | Anchor Scoring   │
│                                                                 │
│  • Aggregate payment outcomes                                   │
│  • Calculate corridor health scores                             │
│  • Rank anchor reliability                                      │
│  • Compute liquidity metrics                                    │
│  • Generate time-series trends                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Data Access Layer                          │
│  Database Queries | Cache | Data Transformation                 │
│                                                                 │
│  • Payment transaction queries                                  │
│  • Order book snapshots                                         │
│  • Historical ledger data                                       │
│  • Time-series aggregations                                     │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Data Source Layer                            │
│  Stellar RPC | Horizon API | Indexed DB | Order Books           │
│                                                                 │
│  • Real-time ledger (RPC)                                       │
│  • Historical transactions (Horizon)                            │
│  • Indexed analytics (Custom DB)                                │
│  • Order book snapshots (DEX)                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🗄️ Data Schema Design

### Core Entities

#### 1. Payments Table
```sql
CREATE TABLE payments (
  id VARCHAR(64) PRIMARY KEY,
  source_account VARCHAR(56) NOT NULL,
  destination_account VARCHAR(56) NOT NULL,
  source_asset VARCHAR(255) NOT NULL,  -- e.g., "USDC:Circle"
  destination_asset VARCHAR(255) NOT NULL,  -- e.g., "BRL:Anclap"
  amount DECIMAL(20,7) NOT NULL,
  actual_amount_received DECIMAL(20,7),
  status ENUM('success', 'failed', 'pending'),
  created_at TIMESTAMP NOT NULL,
  settled_at TIMESTAMP,
  settlement_time_ms INT,
  slippage_percent DECIMAL(5,4),
  routing_path TEXT,  -- JSON: list of hops
  failure_reason VARCHAR(255),
  FOREIGN KEY (source_asset) REFERENCES assets(id),
  FOREIGN KEY (destination_asset) REFERENCES assets(id)
);

CREATE INDEX idx_payments_time ON payments(created_at);
CREATE INDEX idx_payments_assets ON payments(source_asset, destination_asset);
CREATE INDEX idx_payments_status ON payments(status);
```

#### 2. Order Books Table
```sql
CREATE TABLE order_books (
  id INT AUTO_INCREMENT PRIMARY KEY,
  source_asset VARCHAR(255) NOT NULL,
  destination_asset VARCHAR(255) NOT NULL,
  timestamp TIMESTAMP NOT NULL,
  bid_price DECIMAL(20,10),
  ask_price DECIMAL(20,10),
  bid_depth DECIMAL(20,7),  -- Volume available at bid
  ask_depth DECIMAL(20,7),  -- Volume available at ask
  spread_percent DECIMAL(5,4),
  mid_price DECIMAL(20,10),
  UNIQUE KEY (source_asset, destination_asset, timestamp)
);

CREATE INDEX idx_orderbook_assets ON order_books(source_asset, destination_asset);
CREATE INDEX idx_orderbook_time ON order_books(timestamp);
```

#### 3. Anchors Table
```sql
CREATE TABLE anchors (
  id INT AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  stellar_account VARCHAR(56) NOT NULL UNIQUE,
  issuing_assets TEXT NOT NULL,  -- JSON array
  total_volume DECIMAL(20,2),
  total_transactions INT,
  success_transactions INT,
  failed_transactions INT,
  avg_settlement_time_ms INT,
  reliability_score DECIMAL(5,2),  -- 0-100
  status ENUM('green', 'yellow', 'red'),
  created_at TIMESTAMP,
  updated_at TIMESTAMP
);

CREATE INDEX idx_anchors_score ON anchors(reliability_score);
CREATE INDEX idx_anchors_status ON anchors(status);
```

#### 4. Assets Table
```sql
CREATE TABLE assets (
  id VARCHAR(255) PRIMARY KEY,  -- "CODE:ISSUER"
  code VARCHAR(12) NOT NULL,
  issuer_account VARCHAR(56) NOT NULL,
  issuer_name VARCHAR(255),
  total_supply DECIMAL(20,7),
  daily_volume DECIMAL(20,2),
  total_transactions INT,
  circulating_supply DECIMAL(20,7),
  created_at TIMESTAMP,
  updated_at TIMESTAMP
);

CREATE INDEX idx_assets_code ON assets(code);
CREATE INDEX idx_assets_issuer ON assets(issuer_account);
```

#### 5. Corridor Metrics Table
```sql
CREATE TABLE corridor_metrics (
  id INT AUTO_INCREMENT PRIMARY KEY,
  source_asset VARCHAR(255) NOT NULL,
  dest_asset VARCHAR(255) NOT NULL,
  date DATE NOT NULL,
  hour INT,  -- 0-23
  total_payments INT,
  successful_payments INT,
  success_rate_percent DECIMAL(5,2),
  avg_slippage_percent DECIMAL(5,4),
  avg_settlement_time_ms INT,
  volume_usd DECIMAL(20,2),
  liquidity_depth_usd DECIMAL(20,2),
  UNIQUE KEY (source_asset, dest_asset, date, hour)
);

CREATE INDEX idx_corridor_date ON corridor_metrics(date);
CREATE INDEX idx_corridor_assets ON corridor_metrics(source_asset, dest_asset);
```

---

## 🔄 Data Flow & Processing

### Phase 1: Data Collection

**From Stellar RPC:**
```typescript
// Real-time ledger reads every block (~5 seconds)
- Payment operations (source → dest → asset transformations)
- Trade operations (order book updates)
- Trustline changes
- Account creation/updates

// Every 30 seconds: Snapshot order books
GET /orderbook?selling_asset=USDC:Circle&buying_asset=XLM
→ {bids: [], asks: []} → Store in DB
```

**From Stellar Horizon API:**
```typescript
// Hourly historical aggregation
GET /payments?order=desc&limit=200
→ Process payment outcomes
→ Calculate success rates
→ Aggregate by corridor/asset
```

**From Custom Indexing:**
```typescript
// Real-time event stream
- Listen to all payment txs
- Match source + destination assets
- Calculate slippage in real-time
- Store metrics immediately
```

### Phase 2: Metrics Computation

**Success Rate Calculation:**
```typescript
function computeSuccessRate(
  corridor: {source_asset, dest_asset},
  timeWindow: "24h" | "7d" | "30d"
) {
  const payments = db.query(`
    SELECT status FROM payments
    WHERE source_asset = ? AND dest_asset = ?
    AND created_at > NOW() - INTERVAL ?
  `);
  
  const successful = payments.filter(p => p.status === 'success').length;
  return (successful / payments.length) * 100;
}
```

**Corridor Health Score:**
```typescript
function computeCorridorScore(corridor) {
  const successRate = computeSuccessRate(corridor);      // 50% weight
  const liquidityScore = computeLiquidityScore(corridor); // 30% weight
  const slippageScore = computeSlippageScore(corridor);  // 20% weight
  
  return (
    (successRate * 0.5) +
    (liquidityScore * 0.3) +
    (slippageScore * 0.2)
  );
}
```

**Anchor Reliability Score:**
```typescript
function computeAnchorScore(anchor) {
  const txns = db.query(`
    SELECT * FROM payments
    WHERE source_asset LIKE ? OR dest_asset LIKE ?
    AND created_at > NOW() - INTERVAL 30 DAY
  `, [`%:${anchor.account}`, `%:${anchor.account}`]);
  
  const successRate = txns.filter(t => t.status === 'success').length / txns.length;
  const settlementTime = avg(txns.map(t => t.settlement_time_ms));
  const refundRate = computeRefundRate(anchor);
  
  return (
    (successRate * 0.5) +
    ((1 - settlementTime / MAX_TIME) * 0.25) +
    ((1 - refundRate) * 0.15) +
    (anchor.assetDiversity * 0.1)
  );
}
```

### Phase 3: Trend Analysis

**Time-Series Aggregation:**
```sql
-- Daily aggregation of payment volume
INSERT INTO corridor_metrics
SELECT
  source_asset,
  dest_asset,
  DATE(created_at) as date,
  HOUR(created_at) as hour,
  COUNT(*) as total_payments,
  SUM(CASE WHEN status='success' THEN 1 ELSE 0 END) as successful_payments,
  (SUM(CASE WHEN status='success' THEN 1 ELSE 0 END) / COUNT(*)) * 100 as success_rate,
  AVG(slippage_percent) as avg_slippage,
  AVG(settlement_time_ms) as avg_settlement_time,
  SUM(amount) as volume_usd,
  (SELECT MAX(liquidity_depth_usd) FROM order_books ...) as liquidity_depth
FROM payments
WHERE created_at > NOW() - INTERVAL 7 DAY
GROUP BY source_asset, dest_asset, date, hour;
```

---

## 🔌 API Endpoints (Future)

### Core Endpoints

**KPI Metrics**
```
GET /api/metrics/kpi
Response: {
  paymentSuccessRate: 97.8,
  activeCorridors: 142,
  liquidityDepth: 847500000,
  avgSettlementTime: 4.2,
  timestamp: "2025-01-19T..."
}
```

**Corridor Analytics**
```
GET /api/corridors?source=USDC&dest=XLM&days=7
Response: {
  corridors: [
    {
      id: "usdc-xlm",
      source: "USDC:Circle",
      dest: "XLM:native",
      successRate: 99.2,
      avgSlippage: 0.12,
      volume7d: 45000000,
      liquidity: 50000000,
      trend: "up"
    }
  ]
}
```

**Anchor Performance**
```
GET /api/anchors/:stellarAccount
Response: {
  anchor: {
    name: "Circle",
    account: "...",
    reliabilityScore: 98.9,
    status: "green",
    assets: ["USDC", "EURC"],
    metrics: {
      totalVolume: 167000000,
      successRate: 98.9,
      failureRate: 1.1,
      settlements30d: [...],
      trend: [...]
    }
  }
}
```

**Payment Trends**
```
GET /api/payments/trends?days=7
Response: {
  trends: [
    { date: "2025-01-13", volume: 245000000, count: 24100, success: 97.5 },
    { date: "2025-01-14", volume: 235000000, count: 23500, success: 98.1 }
  ]
}
```

**Liquidity History**
```
GET /api/liquidity/history?corridor=usdc-xlm&days=30
Response: {
  history: [
    { timestamp: "2025-01-14T12:00:00Z", depth: 50000000, spreadBps: 12 },
    { timestamp: "2025-01-14T13:00:00Z", depth: 51200000, spreadBps: 11 }
  ]
}
```

---

## 🛠️ Tech Stack Details

### Frontend
| Technology | Purpose | Version |
|-----------|---------|---------|
| React | UI library | 18.3.1 |
| TypeScript | Type safety | 5.8 |
| Vite | Build tool | 5.4.19 |
| Tailwind CSS | Styling | 3.4.17 |
| shadcn-ui | Components | Latest |
| Recharts | Charts | 2.15.4 |
| React Router | Navigation | 6.30.1 |
| TanStack Query | Data fetching | 5.83.0 |

### Backend (Ready for Integration)
| Technology | Purpose |
|-----------|---------|
| Node.js + Express | API server |
| PostgreSQL/MySQL | Primary data store |
| Redis | Caching layer |
| Bull Queue | Job processing |
| Stellar-SDK | Blockchain interaction |

### DevOps (Recommended)
| Technology | Purpose |
|-----------|---------|
| Docker | Containerization |
| GitHub Actions | CI/CD |
| Vercel/Netlify | Frontend hosting |
| AWS RDS | Managed DB |
| Datadog | Monitoring |

---

## 📊 Performance Considerations

### Caching Strategy
```
Level 1: Frontend Cache (React Query)
- KPI metrics: 1 minute
- Corridor data: 5 minutes
- Anchor data: 15 minutes
- Historical data: 1 hour

Level 2: API Cache (Redis)
- Latest snapshots: 30 seconds
- Aggregated data: 5 minutes
- Historical trends: 1 hour

Level 3: Database Indexes
- Payment searches (asset pairs + time)
- Corridor lookups (composite index)
- Anchor searches (by score/status)
```

### Query Optimization
```sql
-- Use materialized views for complex aggregations
CREATE MATERIALIZED VIEW corridor_daily_stats AS
SELECT source_asset, dest_asset, DATE(created_at),
  COUNT(*) as total, SUM(IF(status='success',1,0)) as successful,
  AVG(slippage_percent) as avg_slip
FROM payments
GROUP BY source_asset, dest_asset, DATE(created_at);

-- Refresh hourly or on-demand
REFRESH MATERIALIZED VIEW corridor_daily_stats;
```

---

## 🔒 Security Architecture

### Data Protection
- **Encryption at Rest**: All sensitive data encrypted in DB
- **HTTPS/TLS**: All API communications encrypted
- **Rate Limiting**: API endpoints rate-limited per user
- **CORS**: Frontend origin validation
- **Input Validation**: All inputs sanitized + type-checked

### Privacy
- **Public Ledger Data Only**: No PII stored
- **Aggregate Metrics**: No individual payment details exposed
- **Audit Logging**: All API access logged
- **GDPR Compliant**: No personal data collection

---

## 📈 Scalability Plan

### Phase 1 (Current)
- Single server
- Mock data
- React SPA frontend

### Phase 2 (Q2 2025)
- Docker containerization
- Load-balanced API servers
- Real-time data ingestion
- Redis caching layer

### Phase 3 (Q3 2025)
- Kubernetes orchestration
- Database replication
- Global CDN for assets
- WebSocket for real-time updates

### Phase 4 (Q4 2025)
- Distributed data processing
- Spark/Hadoop for batch analytics
- Advanced monitoring & alerts
- ML-based predictions

---

## 🧪 Testing Strategy

### Unit Tests
```typescript
// test/metrics.test.ts
describe('Metrics Engine', () => {
  it('computes corridor health score correctly', () => {
    const score = computeCorridorScore(testCorridor);
    expect(score).toBeGreaterThan(0);
    expect(score).toBeLessThanOrEqual(100);
  });
});
```

### Integration Tests
```typescript
// test/api.integration.test.ts
describe('API Endpoints', () => {
  it('GET /api/corridors returns proper data', async () => {
    const res = await fetch('/api/corridors?source=USDC');
    expect(res.status).toBe(200);
    expect(res.body.corridors).toBeDefined();
  });
});
```

### E2E Tests
```typescript
// test/e2e/dashboard.spec.ts
describe('Dashboard User Flow', () => {
  it('displays KPIs and updates on refresh', () => {
    cy.visit('/dashboard');
    cy.contains('97.8%').should('be.visible');
    cy.get('[data-testid=refresh-btn]').click();
    cy.wait(1000);
    cy.contains('Success Rate').should('be.visible');
  });
});
```

---

## 📞 Architecture Questions?

For technical details, data integration, or scalability discussions:
- Open an [Architecture Discussion](https://github.com/Ndifreke000/stellar-insights/discussions)
- Check [Features.md](./FEATURES.md) for use cases
- Review [README.md](../README.md) for overview
