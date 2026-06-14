# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,id=astral-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=astral-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=astral-target-gateway,target=/app/target,sharing=locked \
    cargo build --release -p astral_gateway && \
    cp /app/target/release/astral_gateway /tmp/astral_gateway

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /tmp/astral_gateway /usr/local/bin/astral_gateway
EXPOSE 8082
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=60s \
    CMD curl -f http://localhost:8082/health/ready || exit 1
ENTRYPOINT ["/usr/local/bin/astral_gateway"]
