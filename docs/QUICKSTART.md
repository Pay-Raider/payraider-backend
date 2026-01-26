# Quick Start Guide - Analytics Engine

## Prerequisites

- Rust 1.70+ installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- PostgreSQL 14+ running
- `jq` for JSON parsing (optional, for testing)

## 5-Minute Setup

### 1. Database Setup (Docker)

```bash
# Start PostgreSQL container
docker run --name stellar-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=stellar_insights \
  -p 5432:5432 \
  -d postgres:14

# Verify it's running
docker ps | grep stellar-postgres
```

### 2. Backend Configuration

```bash
# Navigate to backend
cd backend

# Copy environment template
cp .env.example .env

# (Optional) Edit if using different database credentials
# nano .env
```

### 3. Build and Run

```bash
# Install dependencies and build
cargo build

# Run the server (migrations run automatically)
cargo run
```

You should see:
```
INFO backend: Connecting to database...
INFO backend: Running database migrations...
INFO backend: Server starting on 127.0.0.1:8080
```

### 4. Test the API

```bash
# Health check
curl http://localhost:8080/health

# Create a test anchor
curl -X POST http://localhost:8080/api/anchors \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Circle",
    "stellar_account": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    "home_domain": "circle.com"
  }' | jq

# List anchors
curl http://localhost:8080/api/anchors | jq
```

### 5. Seed Sample Data (Optional)

```bash
# Run the seed script
./seed_data.sh

# View all seeded anchors
curl http://localhost:8080/api/anchors | jq
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/anchors` | GET | List all anchors |
| `/api/anchors` | POST | Create anchor |
| `/api/anchors/:id` | GET | Get anchor details |
| `/api/anchors/:id/metrics` | PUT | Update metrics |
| `/api/anchors/:id/assets` | GET | List assets |
| `/api/anchors/:id/assets` | POST | Add asset |

## Example Workflow

### 1. Create an Anchor

```bash
ANCHOR_ID=$(curl -s -X POST http://localhost:8080/api/anchors \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Anchor",
    "stellar_account": "GABCDEF123...",
    "home_domain": "myanchor.com"
  }' | jq -r '.id')

echo "Created anchor: $ANCHOR_ID"
```

### 2. Add Assets

```bash
curl -X POST http://localhost:8080/api/anchors/$ANCHOR_ID/assets \
  -H "Content-Type: application/json" \
  -d '{
    "asset_code": "USDC",
    "asset_issuer": "GABCDEF123..."
  }' | jq
```

### 3. Update Metrics

```bash
curl -X PUT http://localhost:8080/api/anchors/$ANCHOR_ID/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "total_transactions": 1000,
    "successful_transactions": 990,
    "failed_transactions": 10,
    "avg_settlement_time_ms": 2000,
    "volume_usd": 100000.00
  }' | jq
```

### 4. View Results

```bash
curl http://localhost:8080/api/anchors/$ANCHOR_ID | jq
```

## Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_compute_metrics_green_status
```

## Troubleshooting

### Database Connection Failed

```bash
# Check if PostgreSQL is running
docker ps

# Check logs
docker logs stellar-postgres

# Restart container
docker restart stellar-postgres
```

### Port Already in Use

```bash
# Check what's using port 8080
lsof -i :8080

# Change port in .env
echo "SERVER_PORT=8081" >> .env
```

### Migration Errors

```bash
# Reset database (WARNING: Deletes all data)
docker exec -it stellar-postgres psql -U postgres -d stellar_insights -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"

# Restart server (migrations will re-run)
cargo run
```

## Next Steps

- See [backend/README.md](../backend/README.md) for detailed API documentation
- See [ANCHOR_METRICS_IMPLEMENTATION.md](./ANCHOR_METRICS_IMPLEMENTATION.md) for implementation details
- Check [ARCHITECTURE.md](./ARCHITECTURE.md) for system design

## Production Deployment

```bash
# Build optimized binary
cargo build --release

# Run in production
DATABASE_URL=postgresql://... \
SERVER_HOST=0.0.0.0 \
SERVER_PORT=8080 \
RUST_LOG=info \
./target/release/backend
```

---

**Ready to integrate with your frontend!** 🚀
