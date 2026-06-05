# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p astral_calculator_api && \
    cp /app/target/release/astral_calculator_api /tmp/astral_calculator_api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /tmp/astral_calculator_api /usr/local/bin/astral_calculator_api
COPY contracts/calculator /app/contracts/calculator
ENV ASTRAL_CALCULATOR_SCHEMAS_DIR=/app/contracts/calculator
ENV ASTRAL_CALCULATOR_OPENAPI_PATH=/app/contracts/calculator/openapi.yaml
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=60s \
    CMD curl -f http://localhost:8080/health/ready || exit 1
ENTRYPOINT ["/usr/local/bin/astral_calculator_api"]
