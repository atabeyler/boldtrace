# Deploying BOLDTRACE on Northflank

The repo builds one multi-stage `Dockerfile` with a separate final stage per
service. Create one Northflank service per stage, all pointing at this repo
with build type **Dockerfile**, build context `/`, Dockerfile path
`Dockerfile`, and the **build target** set per service below.

## Add-ons (create first)

- **Postgres** — any Northflank Postgres addon, or an external instance
  (e.g. Supabase). Copy its connection string for `DATABASE_URL` below.
- **Redis** — any Northflank Redis addon. Copy its connection string for
  `REDIS_URL` below.

## Services

### `product-api` (public HTTP service — this is what users hit)

- Build target: `product-api`
- Port: `8080` (HTTP, public)
- Env vars:
  - `DATABASE_URL` — from the Postgres addon
  - `REDIS_URL` — from the Redis addon
  - `PRODUCT_API_PORT=8080`
  - `COOKIE_SECURE=true`
  - `RUST_LOG=info`
- Serves the web product's static build **and** the `/api/v1/*` HTTP API
  from the same origin, so no CORS configuration is needed. Only set
  `WEB_ORIGIN` if you deploy the web app as a separate Northflank service
  instead — in that case it must be the exact origin the browser sends
  (e.g. `https://app.boldtrace.ai`).
- Runs its own Postgres migrations on startup (idempotent).

### `exchange-client` (background worker, no public port)

- Build target: `exchange-client`
- Env vars: `REDIS_URL`, `EXCHANGE_SYMBOL` (default `BTCUSDT`), `RUST_LOG=info`

### `bot` (background worker, no public port — Telegram bot)

- Build target: `bot`
- Env vars: `TELEGRAM_BOT_TOKEN`, `DATABASE_URL`, `REDIS_URL`, `RUST_LOG=info`

### `backtest` (run on demand — not a long-running service)

- Build target: `backtest`
- Better suited to a Northflank **Job** than an always-on service, since it
  runs once and exits.

## What "finished" means here vs. what still needs a human decision

This gets the app to a real, server-enforced, deployable state: email/password
auth with hashed passwords and HttpOnly session cookies, versioned consent
capture, a real history endpoint reading persisted outcomes, and one
container image per service ready for Northflank's build-target model.

Not covered, and worth deciding deliberately before real users sign up:
- No email verification or password-reset flow (registration is immediate
  activation, matching the "no fake pending states" default in CLAUDE.md,
  but forgetting a password currently has no recovery path).
- No admin-approval gate (also matches the default; enable it only if the
  product needs it).
- No rate limiting on `/api/v1/auth/*` (brute-force protection).
