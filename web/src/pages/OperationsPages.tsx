import type { ReactNode } from 'react';
import { useApi } from '../api/useApi';
import { api } from '../api/client';
import { useI18n } from '../i18n';
import type { Copy } from '../i18n';

export function MarketScanner() {
  const { t } = useI18n();
  const { data, loading, error } = useApi(() => api.scanner(), [], 20000);
  return <Page t={t} title={t.scannerTitle} sub={t.scannerSub}>
    {loading && !data && <section className="panel empty-state" role="status"><h2>{t.scannerLoading}</h2></section>}
    {!loading && (!data || data.length === 0) && <section className="panel empty-state" role={error ? 'alert' : 'status'}><h2>{t.scannerUnavailable}</h2><p>{error}</p></section>}
    {data && data.length > 0 && <section className="ops-card-grid">{data.map(x => <article className="ops-market-card" key={x.symbol}><header><strong>{x.symbol}</strong><span className={x.status === 'live' ? 'healthy' : 'stale'}>{x.status === 'live' ? `● ${t.statusLive}` : x.status === 'stale' ? `○ ${t.itDataStale}` : `○ ${t.statusUnavailable}`}</span></header><div className={`ops-market-decision decision ${x.market?.decision.toLowerCase().replaceAll(' ','-')??'no-trade'}`}>{x.market?.decision ?? '—'}</div><div className="ops-market-stats"><div><span>{t.colConfidence}</span><b>{x.market ? `${x.market.confidence.toFixed(0)}%` : '—'}</b></div><div><span>{t.colRisk}</span><b>{x.market ? `${x.market.risk.toFixed(0)}%` : '—'}</b></div></div></article>)}</section>}
  </Page>;
}

export function AlertsPage() {
  const { t } = useI18n();
  const { data, loading, error } = useApi(() => api.alerts(20), [], 20000);
  return <Page t={t} title={t.alertsTitle} sub={t.alertsSub}><section className="panel alert-list" aria-live="polite">{loading && !data && <p>{t.alertsLoading}</p>}{!loading && (!data || data.length === 0) && <p>{error || t.alertsEmpty}</p>}{data?.map((a, i) => <article key={a.id + '-' + i}><b>{a.severity}</b><strong>{a.symbol}</strong><span>{a.decision} · {a.confidence.toFixed(0)}%</span><small>{new Date(a.createdAt).toLocaleString()}</small></article>)}</section></Page>;
}

export function HistoryPage() {
  const { t } = useI18n();
  const { data, loading, error } = useApi(() => api.history(50), [], 30000);
  return <Page t={t} title={t.historyTitle} sub={t.historySub}><section className="panel"><div className="history-table" aria-live="polite">{loading && !data && <p>{t.historyLoading}</p>}{!loading && (!data || data.length === 0) && <p>{error || t.historyEmpty}</p>}{data?.map(d => <div className="responsive-data-row history-responsive-row" key={d.id}><b data-label={t.colMarket}>{d.symbol}</b><span data-label={t.colDecision}>{d.decision} · {d.confidence.toFixed(0)}%</span><span data-label={t.pcAvgReturn}>{d.realizedReturn !== undefined ? `${d.realizedReturn >= 0 ? '+' : ''}${d.realizedReturn.toFixed(2)}%` : '—'}</span><small data-label={t.colStatus}>{d.horizon} · {d.outcome ?? t.historyPending}</small></div>)}</div></section></Page>;
}

export function SystemHealth() {
  const { t } = useI18n();
  const { data, loading, error } = useApi(() => api.health(), [], 15000);
  return <Page t={t} title={t.healthTitle} sub={t.healthSub}>
    {loading && !data && <section className="panel empty-state" role="status"><h2>{t.healthLoading}</h2></section>}
    {!loading && (!data || data.length === 0) && <section className="panel empty-state" role="alert"><h2>{t.healthUnavailable}</h2><p>{error}</p></section>}
    {data && data.length > 0 && <><section className="metric-grid">{data.map(s => <article key={s.name}><span>{s.name}</span><strong className={statusClass(s.status)}>{s.status.toUpperCase()}</strong><small>{s.freshnessMs !== undefined && s.freshnessMs !== null ? `${Math.round(s.freshnessMs / 1000)}s` : s.latencyMs !== undefined && s.latencyMs !== null ? `${s.latencyMs}ms` : t.healthNominal}</small></article>)}</section><section className="panel"><div className="health-grid">{data.map(s => <div key={s.name}><i className={statusClass(s.status)} aria-hidden="true" /><strong>{s.name}</strong><span>{s.status === 'healthy' ? t.healthOperational : s.status === 'degraded' ? t.healthDegraded : t.healthOffline}</span></div>)}</div></section></>}
  </Page>;
}

function statusClass(status: string) { return status === 'healthy' ? 'health-text' : status === 'degraded' ? 'health-text-warn' : 'health-text-bad'; }

function Page({ t, title, sub, children }: { t: Copy; title: string; sub: string; children: ReactNode }) {
  return <div className="page"><div className="page-head"><div><span className="eyebrow">{t.opsEyebrow}</span><h1>{title}</h1><p>{sub}</p></div></div>{children}</div>;
}
