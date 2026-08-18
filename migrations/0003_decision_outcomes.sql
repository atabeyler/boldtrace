CREATE TABLE IF NOT EXISTS decision_outcomes (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    decision TEXT NOT NULL,
    decided_at_millis BIGINT NOT NULL,
    horizon_minutes INTEGER NOT NULL CHECK (horizon_minutes IN (15,60,240)),
    entry_price DOUBLE PRECISION NOT NULL CHECK (entry_price > 0),
    exit_price DOUBLE PRECISION NOT NULL CHECK (exit_price > 0),
    return_pct DOUBLE PRECISION NOT NULL,
    directional_return_pct DOUBLE PRECISION NOT NULL,
    correct BOOLEAN NOT NULL,
    evaluated_at_millis BIGINT NOT NULL,
    UNIQUE(symbol, decided_at_millis, horizon_minutes)
);
CREATE INDEX IF NOT EXISTS decision_outcomes_symbol_horizon_idx ON decision_outcomes(symbol,horizon_minutes,evaluated_at_millis DESC);
