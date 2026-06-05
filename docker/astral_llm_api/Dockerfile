# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p astral_llm_api && \
    cp /app/target/release/astral_llm_api /tmp/astral_llm_api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /tmp/astral_llm_api /usr/local/bin/astral_llm_api
COPY astral_llm/prompts /app/prompts
COPY contracts/llm /app/contracts/llm
ENV ASTRAL_LLM_PROMPTS_DIR=/app/prompts
ENV ASTRAL_LLM_CONTRACTS_DIR=/app/contracts/llm
EXPOSE 8081
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=60s \
    CMD curl -f http://localhost:8081/health/ready || exit 1
ENTRYPOINT ["/usr/local/bin/astral_llm_api"]
