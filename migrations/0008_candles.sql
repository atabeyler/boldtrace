-- Persisted OHLCV candles, source of truth for the web dashboard's price
-- chart. Populated by product-api: a one-time REST backfill on startup plus
-- an ongoing subscription to exchange-client's `candles:{symbol}:{interval}`
-- Redis pub/sub channel. Never fabricated: an empty table means the chart
-- shows an unavailable/warming-up state, not synthetic candles.
CREATE TABLE IF NOT EXISTS candles (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    interval TEXT NOT NULL,
    open_time_millis BIGINT NOT NULL,
    close_time_millis BIGINT NOT NULL,
    open DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    close DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    UNIQUE(symbol, interval, open_time_millis)
);

CREATE INDEX IF NOT EXISTS candles_symbol_interval_time_idx
    ON candles(symbol, interval, open_time_millis DESC);
