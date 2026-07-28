FROM rust:1.97-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust-http-service /app/wow-server-backend
COPY --from=builder /app/migrations /app/migrations

RUN useradd -r -u 10001 appuser
USER appuser

EXPOSE 3000
ENV RUST_LOG=info
ENTRYPOINT ["/app/wow-server-backend"]
