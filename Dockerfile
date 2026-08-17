# syntax=docker/dockerfile:1

# ---- Builder ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --workspace

# ---- Runtime ----
# Boldtrace builds three binaries (bot, exchange-client, backtest) from one
# workspace; this image ships all of them and CMD picks which one runs, so
# the same image serves every service.
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bot /usr/local/bin/bot
COPY --from=builder /app/target/release/exchange-client /usr/local/bin/exchange-client
COPY --from=builder /app/target/release/backtest /usr/local/bin/backtest

ENV RUST_LOG=info
CMD ["bot"]
