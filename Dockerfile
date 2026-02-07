# Build stage
FROM rust:1.83 AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files first for caching
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx

# Enable SQLx offline mode (no DB access during build)
ENV SQLX_OFFLINE=true

# Copy source code and migrations
COPY src ./src
COPY migrations ./migrations

# Deterministic build with locked dependencies
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/restaurant-backend /app/restaurant-backend
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8000

# Set Rust backtrace for debugging
ENV RUST_BACKTRACE=1

CMD ["/app/restaurant-backend"]
