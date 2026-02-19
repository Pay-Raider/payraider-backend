# ---- Build Stage ----
FROM rust:1.70 as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .
RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/backend /app/backend

EXPOSE 8080

# Docker Health Check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/api/rpc/health || exit 1

CMD ["./backend"]
