-- Persistent decision ledger for Boldtrace market-intelligence decisions.
-- The in-memory DecisionHistory remains useful for low-latency runtime access,
-- while this append-only table provides durable auditability across restarts.
CREATE TABLE decision_ledger (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL CHECK (score >= 0 AND score <= 100),
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL CHECK (confidence >= 0 AND confidence <= 100),
    decided_at_millis BIGINT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX decision_ledger_symbol_decided_at_idx ON decision_ledger (symbol, decided_at_millis DESC);
CREATE INDEX decision_ledger_decided_at_idx ON decision_ledger (decided_at_millis DESC);
COMMENT ON TABLE decision_ledger IS 'Append-only durable audit ledger for market-intelligence decisions.';
COMMENT ON COLUMN decision_ledger.metadata IS 'Extensible structured context for signal quality, risk, data quality, agreement, derivatives, warnings and provenance.';
