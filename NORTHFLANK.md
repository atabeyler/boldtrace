# Deploying BOLDTRACE on Northflank

BOLDTRACE uses the root multi-stage `Dockerfile`. Create one Northflank
service per runtime target, all with build context `/` and Dockerfile path
`Dockerfile`.

## Required add-ons

- PostgreSQL -> `DATABASE_URL`
- Redis -> `REDIS_URL`

Runtime containers execute as the non-root `boldtrace` user.

## `product-api`

- Build target: `product-api`
- Public HTTP port: `8080`
- Required: `DATABASE_URL`, `REDIS_URL`, `PRODUCT_API_PORT=8080`,
  `COOKIE_SECURE=true`
- Email/registration: `RESEND_API_KEY`, `EMAIL_FROM`,
  `ADMIN_NOTIFICATION_EMAIL`
- First-install admin: `ADMIN_BOOTSTRAP_EMAIL`
- Optional scanner universe: `SCAN_SYMBOLS` (`BTCUSDT` by default or `ALL`)

`ADMIN_BOOTSTRAP_EMAIL` is only a first-install bootstrap. The matching
registration is auto-approved/admin only while the database has no existing
admin. Product intelligence endpoints are server-gated by an approved
HttpOnly session and acceptance of the current terms version.

## `exchange-client`

- Build target: `exchange-client`
- Required: `REDIS_URL`
- Provider: `EXCHANGE_PROVIDER=binance` (default) or `bybit`
- Market universe: `EXCHANGE_SYMBOLS=BTCUSDT,ETHUSDT,...` or `ALL`
- Legacy single-symbol `EXCHANGE_SYMBOL` is still accepted.

### Binance adapter

- closed 1m and 5m klines: Binance spot combined stream
- order-book depth: Binance USDⓈ-M perpetual futures
- funding: Binance USDⓈ-M perpetual futures
- open interest: Binance USDⓈ-M futures REST polling

### Bybit adapter

- endpoint: Bybit V5 public `linear` WebSocket
- closed 1m and 5m klines: `kline.1.*` / `kline.5.*` with `confirm=true`
- order-book depth: `orderbook.50.*` with snapshot/delta reconstruction
- funding and open interest: `tickers.*`
- `ALL`: V5 `instruments-info` pagination, filtered to active USDT
  `LinearPerpetual` instruments

Both providers publish identical shared `Candle`, `OrderBookSnapshot`,
`FundingRate` and `OpenInterest` objects into the same Redis channels, so the
score/risk/learning stack is exchange-adapter independent.

The scorer keeps candle intervals separate; 1m is the primary decision
series and 5m contributes multi-timeframe agreement/conflict. Forming klines
are ignored.

## `bot`

- Build target: `bot`
- Required: `TELEGRAM_BOT_TOKEN`, `DATABASE_URL`, `REDIS_URL`

On startup the bot reconstructs per-symbol adaptive weights from persisted
60-minute realized outcomes. Telegram interactive scans use the same final
post-Risk-Guardian decision recorded in the ledger and exposed to the web.

## `backtest`

- Build target: `backtest`
- Prefer a Northflank Job, because it runs once and exits.

Candle-only mode is explicitly labelled `candle-only`. For full-input
historical validation set `BACKTEST_FRAMES_JSON` to aligned candle,
order-book, funding and open-interest observations and configure realistic
execution assumptions:

- `BACKTEST_ROUND_TRIP_FEE_PCT`
- `BACKTEST_SLIPPAGE_PCT`
- `BACKTEST_FUNDING_PCT`

The backtest evaluates LONG and SHORT directionally and uses the next candle
open as the earliest executable entry after a closing-candle signal.

## CI / deployment workflow

GitHub Actions is intentionally configured only for pushes to `main` to avoid
consuming hosted-runner minutes on development branches. Before a production
merge, keep branch work on a non-main branch; once the final commit is moved
to `main`, Rust tests/clippy and web unit/build checks run once.
