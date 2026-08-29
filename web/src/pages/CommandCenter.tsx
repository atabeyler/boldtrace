import { api } from '../api/client';
import { useApi } from '../api/useApi';
import { useIntelligence } from '../api/useIntelligence';
import { hasMeaningfulHistory, historicalPoint, isFresh } from '../domain/market';
import { useI18n } from '../i18n';

export function CommandCenter() {
  const { data, loading, error, refresh } = useIntelligence('BTCUSDT');
  const performance = useApi(() => api.performance('BTCUSDT'), [], 30000);
  const { t } = useI18n();
  const realized = historicalPoint(performance.data);
  const showHistorical = hasMeaningfulHistory(realized);
  const stale = data ? !isFresh(data.freshnessMs) : false;
  const blocked = data?.decision === 'NO TRADE';
  const bannerClass = error ? 'runtime-banner runtime-banner--bad' : stale ? 'runtime-banner runtime-banner--warn' : blocked ? 'runtime-banner runtime-banner--blocked' : 'runtime-banner runtime-banner--ok';
  const bannerTitle = error ? t.statusDegraded : stale ? t.itDataStale : blocked ? `${t.itRiskGuardian} — ${t.itTradeBlocked}` : t.statusNominal;
  const bannerDetail = error ? t.itRefreshFailed : stale ? `${t.itUpdated}: ${Math.max(0, Math.round((data?.freshnessMs ?? 0) / 1000))}s` : blocked ? (data?.reasons[0] ?? t.ccNoExplanation) : t.itDataFresh;

  return <div className="page premium-command-page">
    <div className="page-head">
      <div><span className="eyebrow">{t.ccEyebrow}</span><h1>{t.ccTitle}</h1><p>{t.ccSub}</p></div>
      <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.statusDegraded : stale ? t.itDataStale : data ? t.statusNominal : t.statusConnecting}</button>
    </div>

    {data && <div className={bannerClass} role={error ? 'alert' : 'status'} aria-live="polite"><div><b>{bannerTitle}</b><span>{bannerDetail}</span></div>{blocked && <strong>{data.risk.toFixed(1)}%</strong>}</div>}
    {loading && !data && <section className="panel empty-state" aria-live="polite"><h2>{t.ccConnecting}</h2></section>}
    {!loading && !data && <section className="panel empty-state" role="alert"><h2>{t.ccUnavailable}</h2><p>{error}</p><button onClick={refresh}>{t.retryConnection}</button></section>}

    {data && <>
      <div className="premium-command-layout">
        <section className="panel premium-market-hero">
          <div className="premium-market-head">
            <div className="premium-symbol"><span className="premium-symbol-mark">BT</span><div><strong>{data.symbol}</strong><small>{t.ccLiveMarket}</small></div></div>
            <div className="premium-freshness"><b>{stale ? t.itDataStale : t.itDataFresh}</b><span>{t.itUpdated}: {Math.max(0, Math.round(data.freshnessMs / 1000))}s</span></div>
          </div>
          <div className="premium-price">{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</div>
          <div className="premium-regime"><i/><span>{t.colRegime}: {data.regime}</span></div>
          <div className="premium-decision-row">
            <div className="premium-decision-label"><span>{t.ccCurrentDecision}</span><strong className={`premium-decision-value ${data.decision.toLowerCase().replaceAll(' ', '-')}`}>{data.decision}</strong></div>
            <div className="premium-mini-stats">
              <div className="premium-mini-stat"><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b></div>
              <div className="premium-mini-stat"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
            </div>
          </div>
        </section>

        <section className="panel premium-risk-card">
          <div className="premium-risk-title"><div><span className="eyebrow">{t.ccRiskGuardianLabel}</span><h2>{blocked ? t.itTradeBlocked : t.itDecisionActive}</h2></div><span className={`premium-risk-state ${blocked ? 'blocked' : ''}`}>{blocked ? t.ccBlocked : t.ccActive}</span></div>
          <div className="premium-risk-gauge" style={{ background: `conic-gradient(${blocked ? 'var(--premium-danger)' : 'var(--premium-accent)'} ${Math.max(0, Math.min(100, data.risk)) * 3.6}deg,#18222e 0deg)` }} aria-label={`${t.colRisk} ${data.risk.toFixed(1)}%`} role="img"><div><strong>{data.risk.toFixed(1)}%</strong><span>{t.colRisk}</span></div></div>
          <div className="premium-risk-reason">{data.reasons[0] ?? t.ccNoExplanation}</div>
        </section>
      </div>

      <section className="premium-engine-strip" aria-label={t.ccEngineConsensus}>
        {data.engines.map(engine => <article className="premium-engine-chip" key={engine.name}><span>{engine.name}</span><strong>{engine.score.toFixed(0)}</strong><div className="premium-engine-track" role="progressbar" aria-label={engine.name} aria-valuenow={Math.round(engine.score)} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${Math.max(0, Math.min(100, engine.score))}%` }}/></div></article>)}
      </section>

      <div className="premium-detail-grid">
        <section className="panel">
          <div className="panel-head"><div><span className="eyebrow">{t.ccLatestIntelligence}</span><h2>{t.ccWhyPrefix} {data.decision}?</h2></div><b>{data.engines.length} {t.ccLiveEngines}</b></div>
          <div className="premium-explanation-list">{data.reasons.length ? data.reasons.map((reason,index)=><div className="premium-explanation-item" key={index}><span className="premium-explanation-index">{String(index+1).padStart(2,'0')}</span><span>{reason}</span></div>) : <div className="premium-explanation-item"><span className="premium-explanation-index">01</span><span>{t.ccNoExplanation}</span></div>}</div>
        </section>

        <section className="panel">
          <span className="eyebrow">{t.pcModelPerformance}</span><h2>{data.symbol}</h2>
          <div className="premium-performance-row">
            <div className="premium-performance-cell"><span>{t.ccMetaConfidence}</span><strong>{data.confidence.toFixed(1)}%</strong><small>{t.ccConfidenceSub}</small></div>
            {showHistorical && realized ? <div className="premium-performance-cell"><span>{realized.horizon.toUpperCase()} {t.pcWinRate}</span><strong>{realized.winRate.toFixed(1)}%</strong><small>{realized.samples} {t.pcRealizedSamples}</small></div> : <div className="premium-performance-cell"><span>{t.pcWinRate}</span><strong>—</strong><small>{t.pcNoSamples}</small></div>}
          </div>
        </section>
      </div>
    </>}
  </div>;
}
