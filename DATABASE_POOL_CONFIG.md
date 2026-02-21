# Database Connection Pool Configuration

This document describes the configurable database connection pool settings for the Stellar Insights backend.

## Overview

The database connection pool manages connections to the SQLite database. Proper configuration helps optimize performance under load and prevents resource exhaustion.

## Configuration Options

All pool settings are configured via environment variables in your `.env` file:

```bash
# Maximum number of connections in the pool
DB_POOL_MAX_CONNECTIONS=10

# Minimum number of idle connections to maintain
DB_POOL_MIN_CONNECTIONS=2

# Timeout in seconds when acquiring a connection from the pool
DB_POOL_CONNECT_TIMEOUT_SECONDS=30

# Time in seconds before an idle connection is closed
DB_POOL_IDLE_TIMEOUT_SECONDS=600

# Maximum lifetime in seconds for any connection
DB_POOL_MAX_LIFETIME_SECONDS=1800
```

## Default Values

If environment variables are not set, the following defaults are used:

| Setting | Default | Description |
|---------|---------|-------------|
| `DB_POOL_MAX_CONNECTIONS` | 10 | Maximum concurrent connections |
| `DB_POOL_MIN_CONNECTIONS` | 2 | Minimum idle connections maintained |
| `DB_POOL_CONNECT_TIMEOUT_SECONDS` | 30 | Connection acquisition timeout |
| `DB_POOL_IDLE_TIMEOUT_SECONDS` | 600 | Idle connection timeout (10 minutes) |
| `DB_POOL_MAX_LIFETIME_SECONDS` | 1800 | Maximum connection lifetime (30 minutes) |

## Configuration Guidelines

### Max Connections

- **Low traffic**: 5-10 connections
- **Medium traffic**: 10-20 connections
- **High traffic**: 20-50 connections

For SQLite, keep this relatively low (10-20) since SQLite has limited concurrency.

### Min Connections

Set to 1-2 for most use cases. This ensures connections are ready without consuming excessive resources.

### Connect Timeout

- **Development**: 30 seconds (default)
- **Production**: 10-30 seconds
- Increase if you see frequent timeout errors under load

### Idle Timeout

- **Development**: 600 seconds (10 minutes)
- **Production**: 300-600 seconds (5-10 minutes)
- Shorter timeouts free up resources faster but may cause more connection churn

### Max Lifetime

- **Default**: 1800 seconds (30 minutes)
- Prevents long-lived connections from accumulating issues
- Set to 1-2 hours for production workloads

## Monitoring

### Pool Metrics Endpoint

The application exposes pool metrics at:

```
GET /api/db/pool-metrics
```

Response:
```json
{
  "size": 5,
  "idle": 2
}
```

- `size`: Current number of connections in the pool
- `idle`: Number of idle connections available

### Logs

Pool configuration is logged at startup:

```
Database pool configuration: max_connections=10, min_connections=2, 
connect_timeout=30s, idle_timeout=600s, max_lifetime=1800s
```

## Example Configurations

### Development

```bash
DB_POOL_MAX_CONNECTIONS=5
DB_POOL_MIN_CONNECTIONS=1
DB_POOL_CONNECT_TIMEOUT_SECONDS=30
DB_POOL_IDLE_TIMEOUT_SECONDS=600
DB_POOL_MAX_LIFETIME_SECONDS=1800
```

### Production (Low Traffic)

```bash
DB_POOL_MAX_CONNECTIONS=10
DB_POOL_MIN_CONNECTIONS=2
DB_POOL_CONNECT_TIMEOUT_SECONDS=20
DB_POOL_IDLE_TIMEOUT_SECONDS=300
DB_POOL_MAX_LIFETIME_SECONDS=1800
```

### Production (High Traffic)

```bash
DB_POOL_MAX_CONNECTIONS=20
DB_POOL_MIN_CONNECTIONS=5
DB_POOL_CONNECT_TIMEOUT_SECONDS=15
DB_POOL_IDLE_TIMEOUT_SECONDS=300
DB_POOL_MAX_LIFETIME_SECONDS=3600
```

## Troubleshooting

### Connection Timeout Errors

If you see "connection timeout" errors:
1. Increase `DB_POOL_MAX_CONNECTIONS`
2. Increase `DB_POOL_CONNECT_TIMEOUT_SECONDS`
3. Check for slow queries blocking connections

### High Memory Usage

If memory usage is high:
1. Decrease `DB_POOL_MAX_CONNECTIONS`
2. Decrease `DB_POOL_IDLE_TIMEOUT_SECONDS`
3. Decrease `DB_POOL_MAX_LIFETIME_SECONDS`

### Connection Exhaustion

If you see "too many connections" errors:
1. Increase `DB_POOL_MAX_CONNECTIONS`
2. Optimize queries to release connections faster
3. Check for connection leaks in application code

## Testing

Run pool configuration tests:

```bash
cargo test pool_config
```

## Implementation Details

The pool configuration is implemented in:
- `backend/src/database.rs` - `PoolConfig` struct and implementation
- `backend/src/main.rs` - Pool initialization
- `backend/src/handlers.rs` - Pool metrics endpoint
- `backend/tests/pool_config_test.rs` - Unit tests
