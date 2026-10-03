# PayRaider Backend

**The PayRaider API: check a Stellar payment corridor before paying out.**

It reads recent payments from the Stellar ledger, scores each corridor, and answers `proceed`, `caution`, `hold` or `unknown` for a given payment. It also handles wallet sign-in, API keys and the USDC-paid Pro plan.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-Axum-orange)
![Stellar](https://img.shields.io/badge/Stellar-Horizon%20%7C%20Soroban-black)

Part of [PayRaider](https://github.com/Pay-Raider): [web app](https://github.com/Pay-Raider/payraider-app) · [plugin & SDKs](https://github.com/Pay-Raider/payraider-plugin) · [contracts](https://github.com/Pay-Raider/payraider-contracts) · [mobile](https://github.com/Pay-Raider/payraider-mobile)

---

## The pre-payment check

```bash
curl "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

```json
{
  "decision": "caution",
  "summary": "Marginal on: liquidity. Pay with care or use an alternative corridor.",
  "score": 83.0,
  "checks": [
    { "name": "success_rate", "status": "pass", "detail": "97.0% of recent payments succeeded (minimum 95.0%)" },
    { "name": "liquidity",    "status": "warn", "detail": "payment is 15.0% of $16660 observed liquidity" },
    { "name": "sample_size",  "status": "pass", "detail": "212 recent payments observed (at least 20 for confidence)" },
    { "name": "health_score", "status": "pass", "detail": "corridor health score 83.0 of 100" }
  ],
  "alternatives": [ { "id": "XLM:native->NGN:G...", "success_rate": 99.1, "health_score": 91.0 } ]
}
```

| Check | Pass | Warn | Fail |
| --- | --- | --- | --- |
| Success rate (failed payments included) | ≥ minimum (default 95%) | within 10 points | lower |
| Liquidity vs. payment size | ≤ 10% of observed | ≤ 50% | more |
| Sample size | ≥ 20 recent payments | fewer | never |
| Health score | ≥ 80 | ≥ 60 | lower |

Any fail → `hold`. Any warn → `caution`. Otherwise `proceed`. No recent data → `unknown`.

## Features

- **Pre-payment check**: `GET/POST /api/v1/preflight`. Public; no key needed.
- **Corridor and anchor data**: success rates from Horizon (failed payments included), liquidity, health scores, alternatives.
- **Wallet sign-in**: a challenge signed by the wallet's key (raw Ed25519 or SEP-53, as Freighter's `signMessage` produces).
- **API keys**: owned by a wallet. A key gives its holder its own rate-limit bucket.
- **Pro plan in USDC**: the caller pays an invoice on Stellar and submits the transaction hash. The API verifies the payment on the ledger and raises the key's limit.
- **Multi-signature coordination**: wallets collect verified co-signatures, and the assembled transaction is submitted to Horizon.
- **OpenAPI**: served at `/api/docs`; the spec is in [`docs/openapi.json`](docs/openapi.json).

## Quick start

Requires Rust (stable). SQLite is built in; Redis is optional locally and required for wallet sign-in.

```bash
cp .env.example .env
# Fill in the CHANGE_ME values; the server refuses to start without them:
#   JWT_SECRET=$(openssl rand -base64 48)
#   ENCRYPTION_KEY=$(openssl rand -hex 32)
#   SEP10_SERVER_PUBLIC_KEY=<a Stellar public key you control>
cargo run
```

The API listens on `http://localhost:8080`; check `GET /health`. Set `RPC_MOCK_MODE=true` to run with generated data instead of the live network.

### Docker

```bash
docker build -t payraider-backend .
docker run -p 8080:8080 -v payraider-data:/data --env-file .env payraider-backend
```

Migrations run automatically at startup. The database lives at `/data/payraider.db`. The server listens on `$PORT` when the host sets it.

## Configuration

| Variable | Required | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | yes | SQLite URL, e.g. `sqlite://payraider.db?mode=rwc` |
| `JWT_SECRET`, `ENCRYPTION_KEY` | yes | Session signing and data encryption |
| `STELLAR_NETWORK` | yes | `mainnet` or `testnet` |
| `STELLAR_RPC_URL_*`, `STELLAR_HORIZON_URL_*` | yes | Stellar RPC and Horizon endpoints |
| `SEP10_SERVER_PUBLIC_KEY`, `SEP10_HOME_DOMAIN` | for sign-in | Wallet sign-in identity; the home domain is the web app's host |
| `REDIS_URL` | for sign-in | Challenges and sessions; without Redis, sign-in is refused |
| `CORS_ALLOWED_ORIGINS` | for the web app | The web app's origin |
| `PAYRAIDER_TREASURY_ACCOUNT` | optional | Turns on the USDC Pro plan |
| `PAYRAIDER_PRO_PRICE_USDC`, `PAYRAIDER_PRO_PERIOD_DAYS`, `PAYRAIDER_PRO_LIMIT_PER_MINUTE` | optional | Plan price (default 50), length (30 days) and limit (1,000/min) |

The full list, with comments, is in [`.env.example`](.env.example).

## Rate limits

| Caller | Requests per minute |
| --- | --- |
| No API key | 60 per IP |
| API key | 200 |
| API key on the Pro plan | 1,000 (configurable) |

## Development

```bash
cargo test                       # unit and integration tests
cargo clippy --lib -- -W clippy::all
cargo fmt --all -- --check
```

Compile-time SQL checks use the checked-in `.sqlx` cache. After changing a query or migration, regenerate it:

```bash
DATABASE_URL=sqlite://<migrated db> cargo sqlx prepare -- --all-targets
```

## Project layout

| Path | Contents |
| --- | --- |
| `src/api/` | HTTP handlers: `preflight.rs`, `corridors.rs`, `billing.rs`, `api_keys.rs`, `transactions.rs`, … |
| `src/billing.rs` | USDC invoice and on-chain payment verification |
| `src/multisig.rs` | Transaction hashing, signature verification, Horizon submission |
| `src/auth/` | Wallet sign-in |
| `src/rpc/` | Horizon and Soroban RPC clients |
| `migrations/` | SQLite schema migrations |
| `tests/` | Integration tests |
| `docs/` | Architecture, runbooks, OpenAPI spec |
| `k8s/`, `terraform/`, `elk/`, `monitoring/` | Infrastructure and observability |

## Security

The server never holds user funds or signing keys: billing and multi-signature features only read and submit to the public ledger. Report vulnerabilities as described in [SECURITY.md](SECURITY.md).

## License

[Apache 2.0](LICENSE)
