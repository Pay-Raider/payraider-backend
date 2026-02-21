# Cost Calculator API

The cost calculator estimates total cross-border payment costs and compares routing options.

## Endpoint

`POST /api/cost-calculator/estimate`

## Request Body

```json
{
  "source_currency": "USDC",
  "destination_currency": "NGN",
  "source_amount": 1000,
  "destination_amount": 1500000,
  "routes": ["stellar_dex", "anchor_direct", "liquidity_pool"]
}
```

## Response Highlights

- `source_usd_rate`, `destination_usd_rate`, `mid_market_rate`
- `routes[]` with full fee and slippage breakdown
- `best_route` for the lowest estimated total source cost

## Caching + Conditional Requests

Responses include:

- `Cache-Control: public, max-age=60`
- `ETag`
- `Last-Modified`

Conditional requests are supported using `If-None-Match` and `If-Modified-Since`.
When unchanged, the endpoint returns `304 Not Modified`.

## Example cURL

```bash
curl -i http://127.0.0.1:8080/api/cost-calculator/estimate \
  -H 'Content-Type: application/json' \
  -d '{
    "source_currency":"USDC",
    "destination_currency":"NGN",
    "source_amount":1000,
    "routes":["stellar_dex","anchor_direct","liquidity_pool"]
  }'
```

## Tests

Run:

```bash
cd backend
cargo test --test cost_calculator_test
```
