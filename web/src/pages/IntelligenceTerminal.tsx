import { useIntelligence } from '../api/useIntelligence';
import { isFresh } from '../domain/market';
import { useI18n } from '../i18n';
import type { Copy } from '../i18n';

export function IntelligenceTerminal() {
  const { data, loading, error, refresh } = useIntelligence('BTCUSDT');
  const { t } = useI18n();
  if (loading && !data) return <State t={t} title={t.itConnecting} />;
  if (!data) return <State t={t} title={t.itUnavailable} detail={error || undefined} action={refresh} />;
  const stale = !isFresh(data.freshnessMs);
  const blocked = data.decision === 'NO TRADE';
  const riskClass = blocked ? 'guardian-panel guardian-panel--blocked' : stale || error ? 'guardian-panel guardian-panel--warn' : 'guardian-panel guardian-panel--ok';
  const riskTitle = blocked ? t.itTradeBlocked : stale ? t.itDataStale : error ? t.itConnectionDegraded : t.itDecisionActive;
  const riskDetail = blocked ? (data.reasons[0] ?? t.itNoExplanation) : stale ? `${t.itUpdated}: ${Math.round(data.freshnessMs / 1000)}s` : error ? t.itRefreshFailed : `${t.itDataFresh} · ${t.colRisk} ${data.risk.toFixed(1)}%`;

  return <div className="page">
    <div className="page-head">
      <div><span className="eyebrow">{t.itEyebrowPrefix} {data.symbol}</span><h1>{t.itTitle}</h1><p>{t.itSub}</p></div>
      <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.itConnectionDegraded : stale ? t.itDataStale : t.itDataFresh}</button>
    </div>

    <div className={riskClass} role={error ? 'alert' : 'status'} aria-live="polite">
      <div><span className="eyebrow">{t.itRiskGuardian}</span><b>{riskTitle}</b><p>{riskDetail}</p></div>
      <strong>{data.risk.toFixed(1)}%</strong>
    </div>

    <div className="terminal-grid">
      <section className="panel decision-card">
        <span className="eyebrow">{t.itMetaDecision}</span><h2 className="hero-decision">{data.decision}</h2>
        <div className="decision-stats"><div><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b><small>{t.ccConfidenceSub}</small></div><div><span>{t.colRisk}</span><b>{data.risk.toFixed(1)}%</b></div><div><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div></div>
      </section>
      <section className="panel chart-placeholder">
        <div className="panel-head"><div><span className="eyebrow">{t.itMarketState}</span><h2>{data.symbol}</h2></div><strong>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</strong></div>
        <div className="signal-chart" aria-label={t.itEngineEvidence}>{data.engines.map((e, i) => <i key={e.name + i} style={{ height: `${Math.max(4, e.score)}%` }} title={`${e.name}: ${e.score.toFixed(1)}`} />)}</div>
        <div className="chart-axis"><span>{t.itRuntimeSnapshot}</span><span>{t.colRegime}: {data.regime}</span><span>{t.itUpdated}: {Math.round(data.freshnessMs / 1000)}s</span></div>
      </section>
    </div>
    <div className="dashboard-split">
      <section className="panel"><span className="eyebrow">{t.itEngineEvidence}</span><h2>{t.itConsensusMatrix}</h2><div className="terminal-engines">{data.engines.map(e => <div key={e.name}><strong>{e.name}</strong><span>{e.state} · w {e.weight.toFixed(2)}</span><b>{e.score.toFixed(0)}</b></div>)}</div></section>
      <section className="panel"><span className="eyebrow">{t.itExplainability}</span><h2>{t.itWhyDecision}</h2>{data.reasons.length ? data.reasons.map((r, i) => <p className="terminal-copy" key={i}>{r}</p>) : <p className="terminal-copy">{t.itNoExplanation}</p>}</section>
    </div>
  </div>;
}

function State({ t, title, detail, action }: { t: Copy; title: string; detail?: string; action?: () => void }) {
  return <div className="page"><section className="panel empty-state" role={detail ? 'alert' : 'status'}><span className="eyebrow">{t.itLiveEyebrow}</span><h2>{title}</h2>{detail && <p>{detail}</p>}{action && <button onClick={action}>{t.retryConnection}</button>}</section></div>;
}
