
# 📝 Contribution: OnFinality Soroban RPC Health Check (Rust)

This guide demonstrates how to perform a health check on the **OnFinality Soroban RPC** using Rust. This is useful to verify node availability and response latency before performing analytics or other RPC operations.

---

## **Endpoint**

```text
https://stellar.api.onfinality.io/public
```

**Description:**
OnFinality provides a public RPC endpoint for the Stellar Soroban network. It has shown the fastest response time among available RPC endpoints.

---

## **Rust Implementation Example**

```rust
use reqwest::Client;
use serde_json::json;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let rpc_name = "OnFinality";
    let url = "https://stellar.api.onfinality.io/public";

    println!("Pinging {} ({})....", rpc_name, url);

    // Build the JSON-RPC payload
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "getHealth",
        "id": 1
    });

    let client = Client::new();
    let start_time = Instant::now();

    match client.post(url)
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            let elapsed = start_time.elapsed().as_millis();
            if response.status().is_success() {
                println!("  Status: SUCCESS (HTTP {})", response.status());
                println!("  Response Time: {} ms", elapsed);

                match response.json::<serde_json::Value>().await {
                    Ok(json) => println!("  Response data: {}", serde_json::to_string_pretty(&json).unwrap()),
                    Err(_) => println!("  Response content (non-JSON): {:?}", response.text().await.unwrap_or_default()),
                }
            } else {
                println!("  Status: FAILED (HTTP {})", response.status());
                println!("  Response Time: {} ms", elapsed);
                println!("  Error Message: {:?}", response.text().await.unwrap_or_default());
            }
        }
        Err(err) => {
            println!("  Status: FAILED (Error: {})", err);
        }
    }
    println!("----------------------------------------");
}
```

---

## **Usage**

1. Add `reqwest` and `serde_json` to your `Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "blocking", "tokio"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

2. Run the health check:

```bash
cargo run
```

3. Expected output (example):

```json
Status: SUCCESS (HTTP 200)
Response Time: 123 ms
Response data: {
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "healthy",
    "latestLedger": 51583040,
    "oldestLedger": 51565760,
    "ledgerRetentionWindow": 17281
  }
}
```

---

## **Notes**

* The `getHealth` method **does not require parameters** — sending `params: []` may result in an `-32602 invalid parameters` error.
* Ensure you use **POST** requests; GET requests are **not supported** for JSON-RPC.
* This code uses **async/await** via `tokio` for non-blocking requests.

