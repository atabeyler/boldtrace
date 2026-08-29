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

  return (
    <div className="page">
      <div className="page-head">
        <div><span className="eyebrow">{t.ccEyebrow}</span><h1>{t.ccTitle}</h1><p>{t.ccSub}</p></div>
        <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.statusDegraded : stale ? t.itDataStale : data ? t.statusNominal : t.statusConnecting}</button>
      </div>

      {data && <div className={bannerClass} role={error ? 'alert' : 'status'} aria-live="polite"><div><b>{bannerTitle}</b><span>{bannerDetail}</span></div>{blocked && <strong>{data.risk.toFixed(1)}%</strong>}</div>}

      {loading && !data && <section className="panel empty-state" aria-live="polite"><h2>{t.ccConnecting}</h2></section>}
      {!loading && !data && <section className="panel empty-state" role="alert"><h2>{t.ccUnavailable}</h2><p>{error}</p><button onClick={refresh}>{t.retryConnection}</button></section>}

      {data && <>
        <section className="metric-grid">
          <article><span>{t.ccRegimeLabel}</span><strong>{data.regime}</strong><small>{data.symbol} {t.ccRuntimeState}</small></article>
          <article><span>{t.ccQualityLabel}</span><strong>{data.quality.toFixed(1)}</strong><small>{t.ccQualitySub}</small></article>
          <article className="metric-card--confidence"><span>{t.ccMetaConfidence}</span><strong>{data.confidence.toFixed(1)}%</strong><small>{t.ccConfidenceSub}</small></article>
          <article className={blocked ? 'metric-card--risk-blocked' : 'metric-card--risk'}><span>{t.ccRiskGuardianLabel}</span><strong>{blocked ? t.ccBlocked : t.ccActive}</strong><small>{t.colRisk} {data.risk.toFixed(1)}%</small></article>
        </section>

        <section className="panel">
          <div className="panel-head">
            <div><span className="eyebrow">{t.ccLiveMarket}</span><h2>{t.ccCurrentDecision}</h2></div>
            <b>{data.symbol} · {t.itUpdated} {Math.max(0, Math.round(data.freshnessMs / 1000))}s</b>
          </div>
          <div className="market-table">
            <div className="market-row table-head"><span>{t.colMarket}</span><span>{t.colPrice}</span><span>{t.colDecision}</span><span>{t.ccMetaConfidence}</span><span>{t.colRisk}</span><span>{t.colRegime}</span></div>
            <div className="market-row market-row--responsive">
              <strong data-label={t.colMarket}>{data.symbol}</strong>
              <span data-label={t.colPrice}>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</span>
              <b data-label={t.colDecision} className={`decision ${data.decision.toLowerCase().replaceAll(' ', '-')}`}>{data.decision}</b>
              <span data-label={t.ccMetaConfidence}>{data.confidence.toFixed(1)}%</span>
              <span data-label={t.colRisk}>{data.risk.toFixed(1)}%</span>
              <span data-label={t.colRegime}>{data.regime}</span>
            </div>
          </div>
        </section>

        <div className="dashboard-split">
          <section className="panel"><div className="panel-head"><div><span className="eyebrow">{t.ccEngineConsensus}</span><h2>{data.symbol}</h2></div><b>{data.engines.length} {t.ccLiveEngines}</b></div><div className="engine-bars">{data.engines.map(e => <div key={e.name}><span>{e.name}</span><div role="progressbar" aria-label={e.name} aria-valuenow={Math.round(e.score)} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${Math.max(0, Math.min(100, e.score))}%` }} /></div><b>{e.score.toFixed(0)}</b></div>)}</div></section>
          <section className="panel intelligence-summary">
            <span className="eyebrow">{t.ccLatestIntelligence}</span><h2>{t.ccWhyPrefix} {data.decision}?</h2>
            <ul className="reason-list">{data.reasons.length ? data.reasons.map((r, i) => <li key={i}>{r}</li>) : <li>{t.ccNoExplanation}</li>}</ul>
            <div className="confidence-compare">
              <div className="confidence-block"><span>{t.ccMetaConfidence}</span><strong>{data.confidence.toFixed(1)}%</strong><small>{t.ccConfidenceSub}</small></div>
              {showHistorical && realized && <div className="confidence-block confidence-block--historical"><span>{realized.horizon.toUpperCase()} {t.pcWinRate}</span><strong>{realized.winRate.toFixed(1)}%</strong><small>{realized.samples} {t.pcRealizedSamples}</small></div>}
            </div>
          </section>
        </div>
      </>}
    </div>
  );
}
