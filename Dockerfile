# UNVERIFIED: no Docker build has been run yet (see README "Status").
# Once Cargo.lock is committed, add `--locked` to the build command.
FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* rust-toolchain.toml build.rs ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --bin beauty-backend --bin migrate

# Runtime: no compiler, non-root user, TLS roots only (sqlx uses rustls).
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home app
COPY --from=builder /app/target/release/beauty-backend /usr/local/bin/beauty-backend
# Run as a deploy step before the new version starts: `migrate`
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate
USER app
ENV RUST_LOG=info LOG_FORMAT=json PORT=8000
EXPOSE 8000
CMD ["beauty-backend"]
