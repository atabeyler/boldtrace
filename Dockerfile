# syntax=docker/dockerfile:1

# ---- Rust builder ----
FROM rust:1-slim-bookworm AS rust-builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --workspace

# ---- Web builder ----
FROM node:22-slim AS web-builder
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npm run build

# ---- Runtime base ----
FROM debian:bookworm-slim AS runtime-base
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home boldtrace \
    && mkdir -p /app \
    && chown boldtrace:boldtrace /app
ENV RUST_LOG=info
WORKDIR /app
USER boldtrace

# ---- bot ----
FROM runtime-base AS bot
COPY --from=rust-builder /app/target/release/bot /usr/local/bin/bot
CMD ["bot"]

# ---- exchange-client ----
FROM runtime-base AS exchange-client
COPY --from=rust-builder /app/target/release/exchange-client /usr/local/bin/exchange-client
CMD ["exchange-client"]

# ---- backtest ----
FROM runtime-base AS backtest
COPY --from=rust-builder /app/target/release/backtest /usr/local/bin/backtest
CMD ["backtest"]

# ---- product-api ----
# Serves /api/v1 plus the built web product from the same origin.
FROM runtime-base AS product-api
COPY --from=rust-builder /app/target/release/product-api /usr/local/bin/product-api
COPY --from=web-builder /app/web/dist /app/web-dist
ENV WEB_DIST_DIR=/app/web-dist
EXPOSE 8080
CMD ["product-api"]
