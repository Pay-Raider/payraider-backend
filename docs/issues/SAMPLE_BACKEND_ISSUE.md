# [Backend] Network Context Middleware - Extract and Validate X-Stellar-Network Header

## 🎯 Problem Statement

The backend is currently hardcoded to a single Stellar network with no runtime switching capability. This prevents:
- Mobile clients from switching between testnet and mainnet
- Testing against testnet without separate deployments
- Multi-network support in a single backend instance
- Proper network isolation for different client types

## 💡 Solution

Create an Axum middleware that extracts the `X-Stellar-Network` HTTP header, validates it against allowed values (testnet/mainnet), and injects network context into all downstream request handlers.

## 📁 Files to Create/Modify

### New Files
```
backend/src/middleware/network_context.rs
backend/src/models/network.rs
```

### Modified Files
```
backend/src/main.rs
backend/src/lib.rs
backend/Cargo.toml
```

## 🔧 Technical Implementation

### NetworkContext Struct
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StellarNetwork {
    Testnet,
    Mainnet,
}

impl FromStr for StellarNetwork {
    type Err = NetworkError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "testnet" => Ok(Self::Testnet),
            "mainnet" => Ok(Self::Mainnet),
            _ => Err(NetworkError::InvalidNetwork(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkContext {
    pub network: StellarNetwork,
    pub raw_header_value: String,
}
```

### Middleware Implementation
```rust
pub async fn network_context_middleware(
    mut req: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    let network_header = req.headers()
        .get("X-Stellar-Network")
        .and_then(|h| h.to_str().ok());
    
    let network = match network_header {
        Some(value) => StellarNetwork::from_str(value)
            .map_err(|_| StatusCode::BAD_REQUEST)?,
        None => {
            #[cfg(debug_assertions)]
            { StellarNetwork::Testnet }
            #[cfg(not(debug_assertions))]
            { return Err(StatusCode::BAD_REQUEST); }
        }
    };
    
    let context = NetworkContext {
        network,
        raw_header_value: network_header.unwrap_or("testnet").to_string(),
    };
    
    tracing::info!(network = ?context.network, "Request network context");
    req.extensions_mut().insert(context);
    
    Ok(next.run(req).await)
}
```

## ✅ Acceptance Criteria

### Functional Requirements
- [ ] Middleware extracts `X-Stellar-Network` header from all requests
- [ ] Validates header value is "testnet" or "mainnet" (case-insensitive)
- [ ] Returns HTTP 400 with JSON error for invalid network values
- [ ] Returns HTTP 400 if header missing in production mode
- [ ] Defaults to "testnet" if header missing in development mode
- [ ] Injects `NetworkContext` into Axum request extensions
- [ ] NetworkContext accessible in all downstream handlers
- [ ] Middleware applied globally to all API routes

### Logging & Observability
- [ ] Logs network context with every request
- [ ] Includes request ID in network context logs
- [ ] Adds `network` label to all request metrics
- [ ] Logs warning when defaulting to testnet in dev
- [ ] Logs error when header missing in production

### Code Quality
- [ ] Follows Rust best practices and idioms
- [ ] No `unwrap()` or `expect()` calls
- [ ] Comprehensive rustdoc comments
- [ ] Unit tests achieve >80% code coverage
- [ ] Integration tests with real HTTP requests
- [ ] Clippy passes with no warnings

## 🧪 Testing Strategy

### Unit Tests
- Test parsing "testnet", "TESTNET", "TeStNeT" (case-insensitive)
- Test parsing "mainnet", "MAINNET", "MaInNeT"
- Test invalid values return errors
- Test empty string returns error

### Integration Tests
- Test middleware in full request pipeline
- Test NetworkContext accessible in handlers
- Test error responses properly formatted
- Test missing header in dev vs prod mode

## 🔗 Dependencies

**Depends on**: None (foundational change)
**Required by**: Database Schema Separation, Network-Aware RPC Client

## ⏱️ Estimated Effort

**Total: 3-5 hours**
- Implementation: 1.5 hours
- Unit tests: 1 hour
- Integration tests: 1 hour
- Documentation: 0.5 hours
- Code review: 1 hour

## ✔️ Definition of Done

- [ ] Code implemented and compiles without warnings
- [ ] All unit tests pass (>80% coverage)
- [ ] All integration tests pass
- [ ] Clippy passes with no warnings
- [ ] Rustdoc documentation complete
- [ ] Code reviewed and approved
- [ ] Merged to main branch
- [ ] Deployed to staging environment
