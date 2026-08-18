# BOLDTRACE — Productization Agent Contract

## Project Purpose
BOLDTRACE is a multilingual market-intelligence platform. It ingests live exchange data, evaluates deterministic signal and specialized intelligence engines, produces explainable probabilistic decisions, applies an independent risk/no-trade gate, records realized outcomes, and adapts per-symbol signal weights from observed performance.

BOLDTRACE is an information and statistical screening product. It does not promise returns and must never present probabilistic output as guaranteed investment advice.

## Cardinal Rules
1. Never fabricate live market data, confidence, risk, performance, engine state, health, or outcomes. UI values must come from real backend state; unavailable data is shown as unavailable/warming-up.
2. Risk Guardian has final veto. No presentation layer may bypass a `NO TRADE`/safety decision.
3. Adaptive learning may tune bounded weights only from persisted realized outcomes. No self-modification without evidence, sample-size gates, and bounds.
4. User-facing text must be localized. No hardcoded production UI copy.
5. Authentication, consent, session handling, and authorization are server-enforced. Client-side route guards are UX, never the security boundary.
6. Secrets never enter source control, logs, browser bundles, or analytics.
7. Productization work stays on `agent/productization-v1`. Do not open PRs and do not modify `main`; the owner performs final merge/push workflow.

## Architecture
- `shared` — common market/domain types.
- `exchange-client` — exchange WebSocket/REST ingestion and Redis publication.
- `score-engine` — pure deterministic scoring, market intelligence, specialized engines, confidence calibration, adaptive weights.
- `backtest` — historical validation.
- `bot` — Telegram delivery, consent, alerts, persistence integration.
- `migrations` — PostgreSQL schema.
- `locales` — Fluent localization resources.
- `web` — product web client (React + TypeScript + Vite), created during productization.
- `api` — product HTTP/WebSocket API boundary, created during productization.

Target runtime flow:
`Exchange -> validation/data health -> MarketState -> score/intelligence -> Risk Guardian -> outcome/adaptive learning -> PostgreSQL -> API -> Web + Telegram`.

## Product Surfaces
The V1 web product contains: Command Center, Intelligence Terminal, Engine Matrix, Performance Center, Learning Center, Alert Center, Market Scanner, System Health, account/settings, login and registration.

Every page uses a shared application shell and the same brand footer. Desktop and mobile are first-class layouts; Arabic must render RTL correctly.

## Authentication & Registration Contract
Use the proven Anatolia-Sim flow as the reference pattern, adapted to BOLDTRACE terminology and security requirements:
- Login and registration are two states of one branded authentication surface.
- Registration captures first name, last name, email, password and a BOLDTRACE user code/handle where required by the server model. Do not copy Turkey-specific identity fields unless BOLDTRACE explicitly needs them.
- Login supports a deliberate “remember me” choice; persistent vs session storage behavior must be explicit.
- Server issues/validates authentication and role state. Prefer secure HttpOnly refresh/session cookies; keep short-lived access material out of persistent browser storage where feasible.
- Pending/approval UI may be supported if BOLDTRACE enables administrator approval; otherwise registration becomes immediate activation/email-verification without fake pending states.
- Errors are localized and must not leak whether sensitive account identifiers exist.
- Consent/terms acceptance is versioned and timestamped before intelligence features are usable.

Reference implementation concepts come from `atabeyler/anatolia.bold.sim/client/src/pages/LoginPage.tsx`: cinematic branded entry, login/register mode switching, remember-session behavior, localized status/error text, and server-backed auth calls. Reuse the interaction model, not simulation-specific DNA/genome visuals or fields.

## Six-Language Contract
Supported V1 locales are exactly:
- `tr` Turkish
- `en` English (source locale)
- `de` German
- `fr` French
- `ar` Arabic (RTL)
- `ru` Russian

All web and Telegram user-facing strings must exist in all six languages. New keys are authored in English first and then translated. Arabic layouts use `dir=rtl` at the application/root content boundary and components must remain usable when direction changes.

## Brand System
Brand name: **BOLDTRACE**.
Visual character: institutional market intelligence, premium, technical, restrained, high-contrast dark surfaces with luminous signal accents. Avoid casino/gambling aesthetics, meme-coin styling, fake profit imagery, excessive neon, and decorative values that look like live data.

The BOLDTRACE mark should communicate trace/signal/market structure: a distinctive geometric `B`/trace monogram plus BOLDTRACE wordmark, suitable for app icon, favicon, login hero, navigation rail and reports. Keep vector source in the repository and derive monochrome/small-size variants.

## Shared Footer Contract
Every full page uses one shared footer component, based on the reference `FooterBar` pattern rather than duplicated page markup.

Localized company line:
- TR: `Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.`
- EN: `Bold Military Technology and Defense Industry Inc.`
- DE/FR/AR/RU: localized legal display strings from i18n resources.

Footer includes `BOLDTRACE © 2026`, company name, localized “All rights reserved”, and a concise localized statistical-information/not-investment-advice notice where the surface requires it. Footer placement supports `fixed`, `flow`, and `inline` modes so auth, dashboard, mobile and report surfaces can reuse one component.

## UI/UX Rules
- Never show invented prices, win rates, confidence or engine scores in production mode.
- Loading skeletons are visually distinct from real values.
- `WARMING_UP`, `STALE`, `DEGRADED`, `OFFLINE`, and insufficient-sample states are explicit.
- Decision color is not the only carrier of meaning; always pair color with text/iconography.
- Core decision card shows decision, confidence, risk, data quality, regime and freshness.
- “Why?” explanations are derived from real engine outputs.
- Performance always displays sample count/reliability context.
- Mobile navigation prioritizes Intelligence, Chart, Engines, Performance.
- Accessibility: keyboard navigation, focus states, semantic controls, reduced-motion support, contrast, screen-reader labels.

## Intelligence & Learning Invariants
- Outcome horizons: 15, 60, 240 minutes unless schema/version explicitly changes.
- Neutral/NoTrade outcomes do not contaminate directional accuracy.
- Per-symbol adaptive weights are bounded and normalized.
- Minimum sample gates are mandatory before adaptation.
- Persisted realized outcomes are the source of truth for learning; runtime caches are disposable.
- Startup bootstrap reconstructs learned state from PostgreSQL before claiming the system is fully warmed.
- Stale or missing required data reduces quality and can force NoTrade.

## API Contract
Expose versioned product endpoints under `/api/v1`. Planned surfaces include markets, symbol intelligence, engines, performance, history, alerts, account/session, and health. Use typed response DTOs; never expose internal database rows directly. Real-time updates use a controlled WebSocket/SSE channel with reconnect/backoff and freshness timestamps.

## Data Health
Track freshness independently for candle, order book, funding, open interest, Redis, PostgreSQL and exchange connectivity. Aggregate state is one of `HEALTHY`, `WARMING_UP`, `DEGRADED`, `OFFLINE`. Health status is observable via API and UI and participates in Risk Guardian decisions.

## Coding Conventions
- Code, identifiers, comments, logs, API keys and technical docs are English.
- User-facing copy comes from i18n resources.
- Rust stays deterministic and testable; scoring logic remains free of UI/network concerns.
- TypeScript uses strict typing; avoid `any` in product code.
- Shared UI primitives are preferred over page-specific duplication.
- Conventional commit messages: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.
- Do not add tool/assistant attribution to commits or files.

## Testing & Completion
A feature is complete when its domain logic, API contract, localization, loading/error/empty states, mobile behavior and relevant tests exist. Tests must cover risk vetoes, auth authorization boundaries, locale fallback/RTL, data freshness, outcome learning bounds, and critical API DTOs.

CI may be handled separately by the repository owner during this productization cycle; do not stop feature work merely to manage CI workflows unless explicitly asked. Do not knowingly leave syntax/type errors in touched code.

## Reference Product
`atabeyler/anatolia.bold.sim` is the design/interaction reference for agent instructions, login/register presentation, multilingual patterns and shared footer treatment. BOLDTRACE must adapt those patterns to financial-market intelligence rather than copying simulation-specific domain concepts.