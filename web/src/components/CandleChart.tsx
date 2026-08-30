import { useState } from 'react';
import { api } from '../api/client';
import { useApi } from '../api/useApi';
import type { Candle } from '../api/contracts';
import { useI18n } from '../i18n';

const INTERVALS = ['1m', '5m'] as const;
type Interval = (typeof INTERVALS)[number];

/// Real OHLCV price chart backed by `/api/v1/candles/:symbol`, itself backed
/// by exchange REST/WS history persisted in Postgres. No synthetic candles:
/// an empty or unavailable response renders the same warming-up/unavailable
/// copy the rest of the product uses, never an invented price path.
export function CandleChart({ symbol }: { symbol: string }) {
  const { t } = useI18n();
  const [interval, setInterval] = useState<Interval>('1m');
  const { data, loading, error } = useApi(() => api.candles(symbol, interval, 180), [symbol, interval], 30000);
  const hasData = Boolean(data && data.length > 1);

  return <div className="ws-chart">
    <div className="ws-chart-head">
      <span>{t.ccChartTitle}</span>
      <div className="ws-chart-intervals" role="group" aria-label={t.ccChartTitle}>
        {INTERVALS.map(iv => <button key={iv} className={iv === interval ? 'active' : ''} onClick={() => setInterval(iv)}>{iv.toUpperCase()}</button>)}
      </div>
    </div>
    {hasData && data
      ? <CandleSvg candles={data} />
      : <div className="ws-chart-empty">{loading ? t.ccConnecting : (error || t.ccChartUnavailable)}</div>}
  </div>;
}

function CandleSvg({ candles }: { candles: Candle[] }) {
  const width = 760;
  const height = 260;
  const padding = 8;
  const max = Math.max(...candles.map(c => c.high));
  const min = Math.min(...candles.map(c => c.low));
  const range = Math.max(max - min, 1e-9);
  const slot = (width - padding * 2) / candles.length;
  const bodyWidth = Math.max(1, slot * 0.6);
  const y = (price: number) => padding + (1 - (price - min) / range) * (height - padding * 2);

  return <svg className="ws-chart-svg" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label="OHLC">
    {candles.map((c, i) => {
      const x = padding + i * slot + slot / 2;
      const up = c.close >= c.open;
      const bodyTop = y(Math.max(c.open, c.close));
      const bodyBottom = y(Math.min(c.open, c.close));
      return <g key={c.openTime} className={up ? 'ws-candle-up' : 'ws-candle-down'}>
        <line x1={x} x2={x} y1={y(c.high)} y2={y(c.low)} />
        <rect x={x - bodyWidth / 2} y={bodyTop} width={bodyWidth} height={Math.max(1, bodyBottom - bodyTop)} />
      </g>;
    })}
  </svg>;
}
