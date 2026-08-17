# Boldtrace

Boldtrace is a Telegram-based crypto market scanning and alerting bot that
computes a composite score from real-time exchange data. It is a screening
and information tool — not investment advice.

![Version](https://img.shields.io/badge/version-v0.1.0--alpha-blue)
![License](https://img.shields.io/badge/license-Boldtrace%20Custom-lightgrey)
![Build](https://github.com/atabeyler/boldtrace/actions/workflows/ci.yml/badge.svg)

## License

This project is licensed under the Boldtrace Custom License — see LICENSE
file for details.

## Architecture

Boldtrace is a Cargo workspace made up of independently testable crates:

- `shared` — common types shared across crates (`Candle`, `OrderBookSnapshot`,
  `FundingRate`, `Signal`, `Score`, `User`, `Session`).
- `exchange-client` — Binance/Bybit WebSocket and REST connectivity layer.
- `score-engine` — pure, stateless composite scoring logic.
- `backtest` — Polars-based historical validation of the score engine.
- `bot` — Telegram interface (teloxide) with the authentication/consent flow.

## Setup / Development

```bash
cargo build
cargo test
cargo clippy
```

Configure secrets via a local `.env` file (copy `.env.example`); never
commit secrets to the repository. Database schema lives in `/migrations`
as plain SQL, applied automatically by `bot` on startup via `sqlx::migrate!`
when `DATABASE_URL` is set (falls back to an in-memory store otherwise).

Build and run with Docker:

```bash
docker build -t boldtrace .
docker run --env-file .env boldtrace          # runs the bot (default)
docker run --env-file .env boldtrace exchange-client
docker run --env-file .env boldtrace backtest
```

## Supported languages

en, tr, fr, de, ar, ru
