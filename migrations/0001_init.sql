-- Boldtrace initial schema.

CREATE TABLE users (
    telegram_id BIGINT PRIMARY KEY,
    language TEXT NOT NULL DEFAULT 'en',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One row per consent event, so re-consenting after a terms change keeps
-- the prior acceptance on record rather than overwriting it.
CREATE TABLE consents (
    id BIGSERIAL PRIMARY KEY,
    telegram_id BIGINT NOT NULL REFERENCES users (telegram_id) ON DELETE CASCADE,
    terms_version TEXT NOT NULL,
    consented_at_millis BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX consents_telegram_id_idx ON consents (telegram_id);

-- Composite scores computed by score-engine, kept for history and for the
-- bot's "current score" reads.
CREATE TABLE signals (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    volume_anomaly DOUBLE PRECISION NOT NULL,
    funding_extreme DOUBLE PRECISION NOT NULL,
    order_book_imbalance DOUBLE PRECISION NOT NULL,
    rsi_divergence DOUBLE PRECISION NOT NULL,
    scored_at_millis BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX signals_symbol_scored_at_idx ON signals (symbol, scored_at_millis DESC);

-- One active alarm per (user, symbol); replacing an alarm updates this row
-- rather than inserting a duplicate.
CREATE TABLE alarms (
    id BIGSERIAL PRIMARY KEY,
    telegram_id BIGINT NOT NULL REFERENCES users (telegram_id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (telegram_id, symbol)
);

CREATE INDEX alarms_symbol_idx ON alarms (symbol);

-- One row per backtest run, mirroring backtest::BacktestResult's
-- aggregate fields; the per-signal detail stays in the JSON/CSV export.
CREATE TABLE backtest_results (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    interval TEXT NOT NULL,
    score_threshold DOUBLE PRECISION NOT NULL,
    lookahead_hours BIGINT NOT NULL,
    total_signals BIGINT NOT NULL,
    win_rate DOUBLE PRECISION NOT NULL,
    average_return_pct DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
