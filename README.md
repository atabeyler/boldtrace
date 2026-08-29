# Boldtrace

BOLDTRACE is a multilingual market-intelligence platform. A Rust backend
scores live exchange data, gates it through risk checks, records realized
outcomes and serves decisions through Telegram and a six-language web
product. It is a screening and statistical-information tool — not investment
advice.

![Version](https://img.shields.io/badge/version-v0.1.0--alpha-blue)
![License](https://img.shields.io/badge/license-Boldtrace%20Custom-lightgrey)
![Build](https://github.com/atabeyler/boldtrace/actions/workflows/ci.yml/badge.svg)

## Architecture

BOLDTRACE is a Cargo workspace plus a React web client:

- `shared` — common market/domain types.
- `exchange-client` — public market ingestion and Redis publication. Select
  `EXCHANGE_PROVIDER=binance` (default) or `EXCHANGE_PROVIDER=bybit`.
  Binance uses closed spot klines plus USDⓈ-M perpetual depth/funding/OI;
  Bybit uses its V5 public linear feed for closed klines, perpetual depth,
  funding and open interest. Both adapters publish the same shared domain
  objects and Redis channels.
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

## Confidence semantics

`Meta Confidence` is the live model's internal confidence score. It is **not**
a direct probability that a trade will win. Historical win rate is calculated
only from realized outcomes and is displayed separately with its sample count;
the Command Center suppresses probability-style presentation until at least 30
realized samples exist for the selected horizon.

## Setup / development

```bash
cargo build --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd web
npm ci
npm test
npm run build
```

Copy `.env.example` to a local `.env` and configure secrets locally. Never
commit production credentials. PostgreSQL migrations live in `/migrations`.

### Exchange selection

```bash
# Binance (default)
EXCHANGE_PROVIDER=binance
EXCHANGE_SYMBOLS=BTCUSDT,ETHUSDT

# Bybit V5 linear perpetual adapter
EXCHANGE_PROVIDER=bybit
EXCHANGE_SYMBOLS=BTCUSDT,ETHUSDT

# Discover all active USDT perpetuals from the selected provider at startup
EXCHANGE_SYMBOLS=ALL
```

## Backtest execution assumptions

If only historical candles are supplied, the exported result says
`scope: "candle-only"`; it must not be presented as validation of the complete
live engine. Configure explicit execution assumptions when evaluating net
returns:

```bash
BACKTEST_ROUND_TRIP_FEE_PCT=0.08 \
BACKTEST_SLIPPAGE_PCT=0.04 \
BACKTEST_FUNDING_PCT=0.00 \
cargo run -p backtest
```

For full-input historical validation set `BACKTEST_FRAMES_JSON` to an archive
containing timestamp-aligned candle, order-book, funding and open-interest
observations.

## Docker / deployment

The repository uses a multi-stage `Dockerfile` with one target per deployable
service. Production deployment target is Northflank; see `NORTHFLANK.md`.
Runtime containers execute as the non-root `boldtrace` user.

## CI policy

GitHub Actions runs only on pushes to `main`. The Rust job builds/tests/clippy
checks the workspace and performs a secret scan. The web job uses `npm ci`,
the dependency audit, Node-based unit tests and the production TypeScript/Vite
build. Development branches therefore do not consume Actions minutes.

## Supported languages

`en`, `tr`, `fr`, `de`, `ar`, `ru`

## License

This project is licensed under the Boldtrace Custom License — see `LICENSE`.
