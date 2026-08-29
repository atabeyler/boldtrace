# Boldtrace

BOLDTRACE is a multilingual market-intelligence platform. A Rust backend
scores live exchange data, gates it through risk checks, records realized
outcomes and serves decisions through Telegram and a six-language web
product. It is a screening and statistical-information tool — not investment
advice.

![Version](https://img.shields.io/badge/version-v0.1.0--alpha-blue)
![License](https://img.shields.io/badge/license-Boldtrace%20Custom-lightgrey)
![Build](https://github.com/atabeyler/boldtrace/actions/workflows/ci.yml/badge.svg)

## License

This project is licensed under the Boldtrace Custom License — see `LICENSE`.

## Architecture

BOLDTRACE is a Cargo workspace plus a React web client:

- `shared` — common market/domain types.
- `exchange-client` — Binance public-market ingestion and Redis publication.
  Spot is used for the proven kline feed; USDⓈ-M futures supplies perpetual
  order-book depth, funding and open interest so microstructure and
  derivatives signals refer to the same leveraged venue.
- `score-engine` — deterministic scoring, data-quality/risk primitives,
  market intelligence, specialized engines, confidence calibration and
  bounded adaptive weights.
- `backtest` — historical validation. Candle-only runs are explicitly marked
  `candle-only`; a full-frame runner accepts genuine archived order-book,
  funding and open-interest observations. LONG and SHORT outcomes are
  evaluated directionally and execution-cost assumptions are exported.
- `bot` — Telegram interface, alerts, decision ledger, realized-outcome
  tracking and adaptive-learning loop.
- `product-api` — versioned HTTP API (`/api/v1`) with Argon2 passwords,
  HttpOnly session cookies, server-enforced account/consent access to product
  intelligence, live market data, realized history and health.
- `web` — React + TypeScript + Vite product: Command Center, Intelligence
  Terminal, Engine Matrix, Performance/Learning/Alert Centers, Market Scanner,
  History, System Health and Settings in six languages with Arabic RTL.

Target runtime flow:

`Exchange -> source health -> MarketState -> score -> risk/no-trade gate -> outcome/adaptive learning -> PostgreSQL/Redis -> API -> Web + Telegram`

See `AGENTS.md` / `CLAUDE.md` for the product contract.

## Setup / development

```bash
cargo build --workspace --all-targets
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

cd web
npm ci
npm run build
```

Copy `.env.example` to a local `.env` and configure secrets locally. Never
commit production credentials. PostgreSQL migrations live in `/migrations`
and are applied by the Rust services that own persistence.

## Backtest execution assumptions

The default CLI remains deliberately conservative about claims: if only
historical candles are supplied, the exported result says `scope:
"candle-only"`; it must not be presented as validation of the complete live
engine. Configure explicit execution assumptions when evaluating net returns:

```bash
BACKTEST_ROUND_TRIP_FEE_PCT=0.08 \
BACKTEST_SLIPPAGE_PCT=0.04 \
BACKTEST_FUNDING_PCT=0.00 \
cargo run -p backtest
```

The values above are only an invocation example, not recommended or assumed
exchange fees. Use the actual fee tier, slippage model and funding applicable
to the tested market.

## Docker / deployment

The repository uses a multi-stage `Dockerfile` with one target per deployable
service:

```bash
docker build --target bot -t boldtrace-bot .
docker build --target exchange-client -t boldtrace-exchange-client .
docker build --target backtest -t boldtrace-backtest .
docker build --target product-api -t boldtrace-product-api .
```

Production deployment target is Northflank; see `NORTHFLANK.md` for service
configuration and environment variables.

## Supported languages

`en`, `tr`, `fr`, `de`, `ar`, `ru`
