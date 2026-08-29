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
admin. Once an admin exists, leaving the environment variable configured
cannot mint another bootstrap admin.

Product intelligence endpoints are server-gated by an approved HttpOnly
session and acceptance of the current terms version. `/api/v1/health` and
authentication endpoints remain reachable without product access so clients
can log in and observe service availability.

The web build and API are served from the same origin in the standard image.
Set `WEB_ORIGIN` only when intentionally hosting the browser client on a
separate origin.

## `exchange-client`

- Build target: `exchange-client`
- Required: `REDIS_URL`
- Market universe: `EXCHANGE_SYMBOLS=BTCUSDT,ETHUSDT,...` or `ALL`
- Legacy single-symbol `EXCHANGE_SYMBOL` is still accepted.

Market-source policy:

- closed 1m and 5m klines: Binance spot combined stream
- order-book depth: Binance USDⓈ-M perpetual futures stream
- funding: Binance USDⓈ-M perpetual futures stream
- open interest: Binance USDⓈ-M futures REST

The scorer keeps candle intervals separate; 1m is the primary decision
series and 5m contributes multi-timeframe agreement/conflict. Forming klines
are ignored, so published decisions do not repaint when a candle later
closes. Futures depth/funding/open-interest describe the same leveraged venue.

`ALL` resolves the current USDT perpetual universe at process startup. A
newly listed symbol is picked up on a later restart/redeploy unless dynamic
rediscovery is added in a future release.

## `bot`

- Build target: `bot`
- Required: `TELEGRAM_BOT_TOKEN`, `DATABASE_URL`, `REDIS_URL`

On startup the bot reconstructs per-symbol adaptive weights from persisted
60-minute realized outcomes before starting the live Redis subscriber.
Telegram interactive scans use the same final post-Risk-Guardian decision
that is recorded in the ledger and exposed to the web product.

## `backtest`

- Build target: `backtest`
- Prefer a Northflank Job, because it runs once and exits.

Candle-only mode is explicitly labelled `candle-only` and must not be
presented as full-engine validation. For full-input historical validation set
`BACKTEST_FRAMES_JSON` to a JSON archive containing aligned candle,
order-book, funding and open-interest observations. Configure realistic
execution assumptions with:

- `BACKTEST_ROUND_TRIP_FEE_PCT`
- `BACKTEST_SLIPPAGE_PCT`
- `BACKTEST_FUNDING_PCT`

The backtest evaluates LONG and SHORT directionally and uses the next candle
open as the earliest executable entry after a closing-candle signal.

## Registration and login guards

- New accounts are pending until admin approval, except the one-time initial
  bootstrap admin described above.
- Passwords are Argon2-hashed and sessions use HttpOnly cookies; only the hash
  of each session token is stored in PostgreSQL.
- Current terms acceptance is persisted and required for product intelligence.
- Login location mismatch remains a best-effort additional guard based on
  proxy-provided IP geolocation; provider failure does not become a global
  login outage.

When changing Northflank runtime environment variables through an API, verify
whether the endpoint replaces or merges the environment map before applying a
partial update. Keep database, Redis and cookie settings present on every
production deployment.
