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

  return <div className="page premium-terminal-page">
    <div className="page-head">
      <div><span className="eyebrow">{t.itEyebrowPrefix} {data.symbol}</span><h1>{t.itTitle}</h1><p>{t.itSub}</p></div>
      <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.itConnectionDegraded : stale ? t.itDataStale : t.itDataFresh}</button>
    </div>

    <div className={riskClass} role={error ? 'alert' : 'status'} aria-live="polite">
      <div><span className="eyebrow">{t.itRiskGuardian}</span><b>{riskTitle}</b><p>{riskDetail}</p></div>
      <strong>{data.risk.toFixed(1)}%</strong>
    </div>

    <div className="premium-terminal-layout">
      <section className="panel premium-terminal-decision">
        <span className="eyebrow">{t.itMetaDecision}</span>
        <h2 className={`hero-decision ${data.decision.toLowerCase().replaceAll(' ', '-')}`}>{data.decision}</h2>
        <p className="terminal-copy">{data.reasons[0] ?? t.itNoExplanation}</p>
        <div className="premium-terminal-score-row">
          <div className="premium-terminal-score"><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b></div>
          <div className="premium-terminal-score"><span>{t.colRisk}</span><b>{data.risk.toFixed(1)}%</b></div>
          <div className="premium-terminal-score"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
        </div>
      </section>

      <section className="panel premium-spectrum">
        <div className="panel-head"><div><span className="eyebrow">{t.itEngineEvidence}</span><h2>{data.symbol}</h2></div><strong>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</strong></div>
        <div className="premium-spectrum-bars" aria-label={t.itConsensusMatrix}>
          {data.engines.map((engine,index)=><i key={`${engine.name}-${index}`} style={{ height:`${Math.max(5,Math.min(100,engine.score))}%` }} title={`${engine.name}: ${engine.score.toFixed(1)}`}/>)}
        </div>
        <div className="chart-axis"><span>{t.itRuntimeSnapshot}</span><span>{t.colRegime}: {data.regime}</span><span>{t.itUpdated}: {Math.round(data.freshnessMs / 1000)}s</span></div>
      </section>
    </div>

    <div className="premium-detail-grid">
      <section className="panel">
        <div className="panel-head"><div><span className="eyebrow">{t.itEngineEvidence}</span><h2>{t.itConsensusMatrix}</h2></div><b>{data.engines.length} {t.ccLiveEngines}</b></div>
        <div className="premium-engine-strip">{data.engines.map(engine=><article className="premium-engine-chip" key={engine.name}><span>{engine.name}</span><strong>{engine.score.toFixed(0)}</strong><div className="premium-engine-track" role="progressbar" aria-label={engine.name} aria-valuenow={Math.round(engine.score)} aria-valuemin={0} aria-valuemax={100}><i style={{width:`${Math.max(0,Math.min(100,engine.score))}%`}}/></div><small>{engine.state} · w {engine.weight.toFixed(2)}</small></article>)}</div>
      </section>

      <section className="panel">
        <span className="eyebrow">{t.itExplainability}</span><h2>{t.itWhyDecision}</h2>
        <div className="premium-explanation-list">{data.reasons.length ? data.reasons.map((reason,index)=><div className="premium-explanation-item" key={index}><span className="premium-explanation-index">{String(index+1).padStart(2,'0')}</span><span>{reason}</span></div>) : <div className="premium-explanation-item"><span className="premium-explanation-index">01</span><span>{t.itNoExplanation}</span></div>}</div>
      </section>
    </div>
  </div>;
}

function State({ t, title, detail, action }: { t: Copy; title: string; detail?: string; action?: () => void }) {
  return <div className="page"><section className="panel empty-state" role={detail ? 'alert' : 'status'}><span className="eyebrow">{t.itLiveEyebrow}</span><h2>{title}</h2>{detail && <p>{detail}</p>}{action && <button onClick={action}>{t.retryConnection}</button>}</section></div>;
}
