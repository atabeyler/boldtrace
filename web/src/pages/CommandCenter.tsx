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
  const decisionClass = data?.decision.toLowerCase().replaceAll(' ', '-') ?? 'no-trade';
  const liveStatus = error ? t.statusDegraded : stale ? t.itDataStale : data ? t.statusNominal : t.statusConnecting;

  return <div className="page">
    <div className="page-head">
      <div><span className="eyebrow">{t.ccEyebrow}</span><h1>{t.ccTitle}</h1><p>{t.ccSub}</p></div>
      <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{liveStatus}</button>
    </div>

    {loading && !data && <section className="panel empty-state" aria-live="polite"><h2>{t.ccConnecting}</h2></section>}
    {!loading && !data && <section className="panel empty-state" role="alert"><h2>{t.ccUnavailable}</h2><p>{error}</p><button onClick={refresh}>{t.retryConnection}</button></section>}

    {data && <div className="command-terminal">
      <div className="command-main">
        <section className="command-market-board">
          <div className="command-market-strip">
            <div><strong>{data.symbol}</strong><small>{t.ccLiveMarket}</small></div>
            <div className="command-strip-value"><b>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</b> · {t.colRegime}: {data.regime}</div>
            <div className="command-strip-value">{t.itUpdated}: <b>{Math.max(0, Math.round(data.freshnessMs / 1000))}s</b></div>
            <div className="command-strip-value">{t.ccQualityLabel}: <b>{data.quality.toFixed(1)}%</b></div>
          </div>

          <div className="command-core">
            <div className="command-decision-zone">
              <div>
                <div className="command-decision-kicker"><i/><span>{t.ccCurrentDecision}</span></div>
                <div className={`command-decision ${decisionClass}`}>{data.decision}</div>
                <div className="command-price">{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</div>
              </div>
              <div className="command-stat-cluster">
                <div className="command-stat"><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b></div>
                <div className="command-stat"><span>{t.colRisk}</span><b>{data.risk.toFixed(1)}%</b></div>
                <div className="command-stat"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
              </div>
            </div>

            <div className="command-spectrum-zone">
              <div className="command-spectrum-head">
                <div><span className="eyebrow">{t.ccEngineConsensus}</span><h2>{data.symbol} · {data.engines.length} {t.ccLiveEngines}</h2></div>
                <small>{stale ? t.itDataStale : t.itDataFresh}<br/>{t.colRegime}: {data.regime}</small>
              </div>
              <div className="command-spectrum" aria-label={t.ccEngineConsensus}>
                {data.engines.map(engine => <div className="command-spectrum-col" key={engine.name} title={`${engine.name}: ${engine.score.toFixed(1)}`}>
                  <div className="command-spectrum-bar" style={{height:`${Math.max(5,Math.min(100,engine.score))}%`}}/>
                  <span>{engine.name}</span><b>{engine.score.toFixed(0)}</b>
                </div>)}
              </div>
              <div className="command-spectrum-footer"><span>{t.ccEngineConsensus}</span><b>{data.engines.length} {t.ccLiveEngines}</b></div>
            </div>
          </div>
        </section>

        <div className="command-analysis-grid">
          <section className="panel command-analysis-card">
            <div className="panel-head"><div><span className="eyebrow">{t.ccLatestIntelligence}</span><h2>{t.ccWhyPrefix} {data.decision}?</h2></div><b>{data.symbol}</b></div>
            <div className="command-reasons">{data.reasons.length ? data.reasons.map((reason,index)=><div className="command-reason" key={index}><span className="command-reason-index">{String(index+1).padStart(2,'0')}</span><span>{reason}</span></div>) : <div className="command-reason"><span className="command-reason-index">01</span><span>{t.ccNoExplanation}</span></div>}</div>
          </section>

          <section className="panel command-engine-ledger">
            <span className="eyebrow">{t.ccEngineConsensus}</span><h2>{data.symbol}</h2>
            <div className="command-engine-ledger-list">
              {data.engines.map(engine=><div className="command-engine-ledger-row" key={engine.name}><strong>{engine.name}</strong><span>{engine.state}</span><span>w {engine.weight.toFixed(2)}</span><b>{engine.score.toFixed(0)}</b></div>)}
            </div>
          </section>
        </div>
      </div>

      <aside className="command-rail">
        <section className="command-risk-panel">
          <div className="command-risk-header"><div><span className="eyebrow">{t.ccRiskGuardianLabel}</span><h2>{blocked ? t.itTradeBlocked : t.itDecisionActive}</h2></div><span className={`command-risk-state ${blocked ? 'blocked' : ''}`}>{blocked ? t.ccBlocked : t.ccActive}</span></div>
          <div className="command-risk-meter"><div className="command-risk-meter-head"><strong>{data.risk.toFixed(1)}%</strong><span>{t.colRisk}</span></div><div className="command-risk-track"><i style={{width:`${Math.max(0,Math.min(100,data.risk))}%`}}/></div></div>
          <div className="command-risk-copy">{data.reasons[0] ?? t.ccNoExplanation}</div>
        </section>

        <section className="panel command-performance-panel">
          <span className="eyebrow">{t.pcModelPerformance}</span><h2>{data.symbol}</h2>
          <div className="command-performance-grid">
            <div className="command-performance-cell"><span>{t.ccMetaConfidence}</span><strong>{data.confidence.toFixed(1)}%</strong><small>{t.ccConfidenceSub}</small></div>
            {showHistorical && realized ? <div className="command-performance-cell"><span>{realized.horizon.toUpperCase()} {t.pcWinRate}</span><strong>{realized.winRate.toFixed(1)}%</strong><small>{realized.samples} {t.pcRealizedSamples}</small></div> : <div className="command-performance-cell"><span>{t.pcWinRate}</span><strong>—</strong><small>{t.pcNoSamples}</small></div>}
          </div>
        </section>

        <section className="panel command-performance-panel">
          <span className="eyebrow">{t.itMarketState}</span><h2>{liveStatus}</h2>
          <div className="command-performance-grid">
            <div className="command-performance-cell"><span>{t.colRegime}</span><strong>{data.regime}</strong><small>{data.symbol}</small></div>
            <div className="command-performance-cell"><span>{t.itUpdated}</span><strong>{Math.max(0, Math.round(data.freshnessMs / 1000))}s</strong><small>{stale ? t.itDataStale : t.itDataFresh}</small></div>
          </div>
        </section>
      </aside>
    </div>}
  </div>;
}
