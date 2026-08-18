# Boldtrace

BOLDTRACE is a multilingual market-intelligence platform. A Rust backend
scores live exchange data, gates it through an independent risk check, and
delivers decisions via a Telegram bot and a six-language web product. It is
a screening and information tool — not investment advice.

![Version](https://img.shields.io/badge/version-v0.1.0--alpha-blue)
![License](https://img.shields.io/badge/license-Boldtrace%20Custom-lightgrey)
![Build](https://github.com/atabeyler/boldtrace/actions/workflows/ci.yml/badge.svg)

## License

This project is licensed under the Boldtrace Custom License — see LICENSE
file for details.

## Architecture

Boldtrace is a Cargo workspace made up of independently testable crates,
plus a web frontend:

- `shared` — common types shared across crates (`Candle`, `OrderBookSnapshot`,
  `FundingRate`, `Signal`, `Score`, `User`, `Session`, `LiveIntelligence`).
- `exchange-client` — Binance/Bybit WebSocket and REST connectivity layer,
  publishing to Redis.
- `score-engine` — pure, stateless composite scoring logic, market
  intelligence, specialized engines, confidence calibration and adaptive
  weights.
- `backtest` — Polars-based historical validation of the score engine.
- `bot` — Telegram interface (teloxide) with the authentication/consent flow,
  persisted decision ledger and adaptive learning loop.
- `product-api` — versioned HTTP API (`/api/v1`) for the web product:
  email/password auth (Argon2, HttpOnly session cookies), live intelligence,
  realized decision history, and health. Serves the built `web` app as
  static files from the same origin in production.
- `web` — the product web client (React + TypeScript + Vite): Command
  Center, Intelligence Terminal, Engine Matrix, Performance/Learning/Alert
  Centers, Market Scanner, History, System Health, Settings, in six
  languages with Arabic RTL support.

See `CLAUDE.md` / `AGENTS.md` for the full product contract and rules this
codebase is held to.

## Setup / Development

```bash
cargo build
cargo test
cargo clippy --workspace --all-targets -- -D warnings

cd web && npm install && npm run build
```

Configure secrets via a local `.env` file (copy `.env.example`); never
commit secrets to the repository. Database schema lives in `/migrations`
as plain SQL, applied automatically by `bot` and `product-api` on startup
via `sqlx::migrate!` when `DATABASE_URL` is set (`bot` falls back to an
in-memory store otherwise).

## Docker / deployment

The `Dockerfile` is one multi-stage build with a separate final stage per
deployable service, selected with `--target`:

```bash
docker build --target bot -t boldtrace-bot .
docker build --target exchange-client -t boldtrace-exchange-client .
docker build --target backtest -t boldtrace-backtest .
docker build --target product-api -t boldtrace-product-api .   # also builds web/

docker run --env-file .env boldtrace-bot
docker run --env-file .env boldtrace-exchange-client
docker run --env-file .env boldtrace-backtest
docker run --env-file .env -p 8080:8080 boldtrace-product-api
```

Production deployment target is [Northflank](https://northflank.com) — see
`NORTHFLANK.md` for the per-service build target, ports and env vars.

## Supported languages

en, tr, fr, de, ar, ru
