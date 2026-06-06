# syntax=docker/dockerfile:1.7

FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p astral_llm_worker && \
    cp /app/target/release/astral_llm_worker /tmp/astral_llm_worker

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /tmp/astral_llm_worker /usr/local/bin/astral_llm_worker
COPY astral_llm/prompts /app/prompts
COPY contracts /app/contracts
ENV ASTRAL_LLM_PROMPTS_DIR=/app/prompts
ENV ASTRAL_LLM_CONTRACTS_DIR=/app/contracts/llm
ENTRYPOINT ["/usr/local/bin/astral_llm_worker"]
