# Deploying PayRaider on Render

Each deployable repository has a `render.yaml` Render Blueprint at its root.
Deploy the API first, because the other two need its URL.

| Repository | Services | Plan |
| --- | --- | --- |
| [payraider-backend](https://github.com/Pay-Raider/payraider-backend) | `payraider-api` (Docker, 1 GB disk for SQLite) and `payraider-redis` | API: Starter (a persistent disk needs a paid instance). Redis: Free |
| [payraider-app](https://github.com/Pay-Raider/payraider-app) | `payraider-web` (Next.js) | Free |
| [payraider-plugin](https://github.com/Pay-Raider/payraider-plugin) | `payraider-mcp` (hosted MCP server, Docker) | Free |

Free Render services sleep when idle and take a few seconds to wake up. That
is fine for a demo. Keep the API on a paid plan for real traffic.

## 1. API and Redis

1. Sign in at render.com and connect GitHub with access to the `Pay-Raider`
   organisation.
2. **New > Blueprint**, pick `Pay-Raider/payraider-backend`, branch `main`.
3. Fill in the variables Render asks for, then **Apply**:

| Variable | Value |
| --- | --- |
| `STELLAR_RPC_URL_MAINNET` | A Soroban RPC URL for mainnet from an RPC provider |
| `ENCRYPTION_KEY` | Output of `openssl rand -hex 32` (exactly 64 hex characters) |
| `SEP10_SERVER_PUBLIC_KEY` | A Stellar public key (`G...`) you control. Only the public key is needed |
| `SEP10_HOME_DOMAIN` | The web app's host name, e.g. `payraider-web.onrender.com` |
| `CORS_ALLOWED_ORIGINS` | The web app's origin, e.g. `https://payraider-web.onrender.com` |
| `PAYRAIDER_TREASURY_ACCOUNT` | Your Stellar account for Pro plan payments, or empty to keep everything free |

Render generates `JWT_SECRET` and wires `REDIS_URL` to `payraider-redis`.

You will not know the web app's URL yet. Enter the expected
`payraider-web.onrender.com` values, and correct them after step 2 if Render
gives the app a different name.

Check it:

```bash
curl https://payraider-api.onrender.com/health
curl "https://payraider-api.onrender.com/api/v1/preflight?source_asset=USDC&destination_asset=XLM"
```

## 2. Web app

**New > Blueprint**, pick `Pay-Raider/payraider-app`, and set:

| Variable | Value |
| --- | --- |
| `NEXT_PUBLIC_API_URL` | The API's URL, e.g. `https://payraider-api.onrender.com` |

`NEXT_PUBLIC_API_URL` is compiled into the build, so redeploy the app after
changing it.

`SEP10_HOME_DOMAIN` on the API must equal the host name people use to open the
web app. Wallet sign-in sends that host name and the API rejects any other.

## 3. MCP server

**New > Blueprint**, pick `Pay-Raider/payraider-plugin`, and set:

| Variable | Value |
| --- | --- |
| `PAYRAIDER_BASE_URL` | The API's URL, e.g. `https://payraider-api.onrender.com` |

Render generates `PAYRAIDER_MCP_AUTH_TOKEN`. Copy it from the service's
**Environment** tab; MCP clients send it as `Authorization: Bearer <token>`.

```bash
curl https://payraider-mcp.onrender.com/healthz
```

## Other hosts

The API and the MCP server are plain Docker images:

```bash
# in payraider-backend
docker build -t payraider-api .
docker run -p 8080:8080 -v payraider-data:/data --env-file .env payraider-api

# in payraider-plugin
docker build -f sdk/mcp-server/Dockerfile -t payraider-mcp sdk
```

The API keeps its database at `/data/payraider.db`; mount a volume there. It
listens on `$PORT` if the host sets one, otherwise 8080.
