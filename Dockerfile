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
    && rm -rf /var/lib/apt/lists/*
ENV RUST_LOG=info

# ---- bot ----
# Northflank service build target: bot
FROM runtime-base AS bot
COPY --from=rust-builder /app/target/release/bot /usr/local/bin/bot
CMD ["bot"]

# ---- exchange-client ----
# Northflank service build target: exchange-client
FROM runtime-base AS exchange-client
COPY --from=rust-builder /app/target/release/exchange-client /usr/local/bin/exchange-client
CMD ["exchange-client"]

# ---- backtest ----
# Northflank service build target: backtest
FROM runtime-base AS backtest
COPY --from=rust-builder /app/target/release/backtest /usr/local/bin/backtest
CMD ["backtest"]

# ---- product-api ----
# Northflank service build target: product-api. Serves the versioned HTTP
# API under /api/v1 and the built web product as static files from the
# same origin, so the browser never needs cross-site cookies.
FROM runtime-base AS product-api
COPY --from=rust-builder /app/target/release/product-api /usr/local/bin/product-api
COPY --from=web-builder /app/web/dist /app/web-dist
WORKDIR /app
ENV WEB_DIST_DIR=/app/web-dist
EXPOSE 8080
CMD ["product-api"]
