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
  - `RESEND_API_KEY` — from your Resend account; required to actually send
    registration/approval/location-alert emails. Omitting it doesn't block
    registration or login, it just skips the email (logged).
  - `ADMIN_NOTIFICATION_EMAIL` — where new-registration and login-location
    alerts are sent.
  - `ADMIN_BOOTSTRAP_EMAIL` — the one email address that, on first
    registration, is auto-approved and made admin (so there's always a way
    in before any admin exists to approve one). Should match the address
    you register with first.
  - `EMAIL_FROM` — the Resend-verified sender, e.g.
    `BOLDTRACE <noreply@yourdomain>`.
  - `SCAN_SYMBOLS` (optional, default `BTCUSDT`) — comma-separated symbols
    the Market Scanner and System Health's exchange-feed check look at, or
    `ALL` to discover every symbol currently publishing live intelligence
    directly from Redis (no list to keep in sync — whatever
    `exchange-client` is actually tracking shows up automatically). With a
    fixed list, only symbols `exchange-client`'s own `EXCHANGE_SYMBOLS` is
    actually ingesting will show live data; anything else honestly reports
    `unavailable` rather than a fabricated row.
- Serves the web product's static build **and** the `/api/v1/*` HTTP API
  from the same origin, so no CORS configuration is needed. Only set
  `WEB_ORIGIN` if you deploy the web app as a separate Northflank service
  instead — in that case it must be the exact origin the browser sends
  (e.g. `https://app.boldtrace.ai`).
- Runs its own Postgres migrations on startup (idempotent).
- Registration requires admin approval (except the bootstrap admin above);
  see `## Registration & login guards` below.
- **When updating env vars via the Northflank API**, always PUT/POST the
  full combined set — the `runtime-environment` endpoint *replaces* the
  whole map rather than merging into it, so sending only the new keys
  silently wipes `DATABASE_URL`/`REDIS_URL` and takes the service down.

### `exchange-client` (background worker, no public port)

- Build target: `exchange-client`
- Env vars: `REDIS_URL`, `EXCHANGE_SYMBOLS` (comma-separated, e.g.
  `BTCUSDT,ETHUSDT,SOLUSDT`; `ALL` to track every currently trading USDT
  perpetual future; default `BTCUSDT`; the older single-symbol
  `EXCHANGE_SYMBOL` still works too), `RUST_LOG=info`
- A single instance tracks every symbol in `EXCHANGE_SYMBOLS` — candles,
  order book depth and funding rate are all sourced from the USDT-M futures
  market (not spot), so every symbol has all four data types the scorer
  needs. Streams are grouped across as many combined WebSocket connections
  as needed to stay under Binance's 1024-streams-per-connection limit, so
  adding a symbol (or using `ALL`, currently ~400 symbols) does **not**
  require a second worker — it does add real load to the one worker (more
  WebSocket subscriptions, more open-interest REST polls, more decisions
  persisted to Postgres per minute), so size the service's CPU/RAM for the
  list you configure, especially `ALL`.
- `ALL` re-resolves the live symbol list from Binance once at startup, not
  continuously — a newly listed perpetual is picked up on the next
  restart/redeploy, not immediately.
- `product-api`'s `SCAN_SYMBOLS=ALL` discovers the live set from Redis
  directly, so it doesn't need to be kept in sync with a fixed
  `EXCHANGE_SYMBOLS` list.

### `bot` (background worker, no public port — Telegram bot)

- Build target: `bot`
- Env vars: `TELEGRAM_BOT_TOKEN`, `DATABASE_URL`, `REDIS_URL`, `RUST_LOG=info`

### `backtest` (run on demand — not a long-running service)

- Build target: `backtest`
- Better suited to a Northflank **Job** than an always-on service, since it
  runs once and exits.

## Registration & login guards

- **Admin approval**: every registration except the `ADMIN_BOOTSTRAP_EMAIL`
  account lands in `pending` status. It can't log in until an admin approves
  it from the in-app Admin panel (visible in the sidebar to `isAdmin`
  accounts). Approve/reject sends the applicant an email (best-effort, needs
  `RESEND_API_KEY`/`EMAIL_FROM` configured) and, on new registration, the
  admin gets notified too (needs `ADMIN_NOTIFICATION_EMAIL`).
- **Registration fields**: first/last name, a user-chosen user code
  (4-20 alphanumeric chars), country, a national ID/citizenship number, email
  and password are all required.
- **Login-time location guard**: on every login, the server resolves the
  request's country from its IP (via a third-party IP-geolocation lookup)
  and compares it against the account's registered country. A mismatch
  blocks the login (`403 location_mismatch`), records an alert, and emails
  the admin. The admin can "allow" the alert from the Admin panel, which
  grants the account a 24-hour exemption so a traveling user isn't
  permanently locked out. If the IP lookup itself fails or can't be
  performed (e.g. local dev with no forwarding proxy), the check **fails
  open** — login proceeds — since a third-party outage blocking every login
  would be worse than skipping one check.

## What "finished" means here vs. what still needs a human decision

This gets the app to a real, server-enforced, deployable state: email/password
auth with hashed passwords and HttpOnly session cookies, versioned consent
capture, a real history endpoint reading persisted outcomes, admin-approval
gated registration with a login-time location guard, and one container image
per service ready for Northflank's build-target model.

Not covered, and worth deciding deliberately before real users sign up:
- No email verification or password-reset flow (forgetting a password
  currently has no recovery path).
- The location guard uses a free third-party IP-geolocation API
  (`ipapi.co`) with no API key — fine for launch volume, but revisit if
  login volume grows past its rate limit or accuracy becomes a problem.
