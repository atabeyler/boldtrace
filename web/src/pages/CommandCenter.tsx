import { api } from '../api/client';
import { useApi } from '../api/useApi';
import { useIntelligence } from '../api/useIntelligence';
import { hasMeaningfulHistory, healthStatusClass, historicalPoint, isFresh, scannerStatusLabel } from '../domain/market';
import { useI18n } from '../i18n';

export function CommandCenter() {
  const { data, loading, error, refresh } = useIntelligence('BTCUSDT');
  const performance = useApi(() => api.performance('BTCUSDT'), [], 30000);
  const scanner = useApi(() => api.scanner(), [], 20000);
  const history = useApi(() => api.history(30), [], 30000);
  const health = useApi(() => api.health(), [], 15000);
  const { t } = useI18n();
  const realized = historicalPoint(performance.data);
  const showHistorical = hasMeaningfulHistory(realized);
  const stale = data ? !isFresh(data.freshnessMs) : false;
  const blocked = data?.decision === 'NO TRADE';
  const decisionClass = data?.decision.toLowerCase().replaceAll(' ', '-') ?? 'no-trade';

  if (loading && !data) return <div className="page workstation-page"><section className="panel empty-state"><h2>{t.ccConnecting}</h2></section></div>;
  if (!data) return <div className="page workstation-page"><section className="panel empty-state" role="alert"><h2>{t.ccUnavailable}</h2><p>{error}</p><button onClick={refresh}>{t.retryConnection}</button></section></div>;

  return <div className="page workstation-page">
    <div className="workstation-grid">
      <aside className="ws-pane ws-watchlist">
        <div className="ws-pane-head"><strong>{t.navScanner}</strong><span>{scanner.data?.length ?? 0}</span></div>
        <div className="ws-watch-tabs"><span className="active">{t.colMarket}</span><span>{t.colStatus}</span></div>
        <div className="ws-watchlist-body">
          {scanner.data?.map(item => <div className="ws-watch-row" key={item.symbol}>
            <div className="ws-watch-symbol"><strong>{item.symbol}</strong><small>{scannerStatusLabel(item.status, t)}</small></div>
            <div className={`ws-watch-decision decision ${item.market?.decision.toLowerCase().replaceAll(' ','-') ?? 'no-trade'}`}>{item.market?.decision ?? '—'}</div>
            <div className="ws-watch-risk">{item.market ? `${item.market.risk.toFixed(0)}%` : '—'}</div>
          </div>)}
          {!scanner.loading && (!scanner.data || scanner.data.length === 0) && <div className="ws-side-copy" style={{padding:10}}>{scanner.error || t.scannerUnavailable}</div>}
        </div>
      </aside>

      <main className="ws-pane ws-center">
        <div className="ws-marketbar">
          <div className="ws-market-identity"><strong>{data.symbol}</strong><small>{stale ? t.itDataStale : t.itDataFresh}</small></div>
          <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.itConnectionDegraded : stale ? t.itDataStale : t.itDataFresh}</button>
          <div className="ws-market-metric"><span>{t.colMarket}</span><b>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</b></div>
          <div className="ws-market-metric"><span>{t.colRegime}</span><b>{data.regime}</b></div>
          <div className="ws-market-metric"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
          <div className="ws-market-metric"><span>{t.itUpdated}</span><b>{Math.max(0, Math.round(data.freshnessMs / 1000))}s</b></div>
        </div>

        <div className="ws-main-workbench">
          <section className="ws-evidence-board">
            <div className="ws-evidence-header"><div><span>{t.ccEngineConsensus}</span><strong>{data.engines.length} {t.ccLiveEngines}</strong></div><b>{data.confidence.toFixed(1)}%</b></div>
            <div className="ws-spectrum" aria-label={t.ccEngineConsensus}>
              {data.engines.map(engine => <div className="ws-spectrum-col" key={engine.name} title={`${engine.name}: ${engine.score.toFixed(1)}`}>
                <div className="ws-spectrum-bar-wrap"><i className="ws-spectrum-bar" style={{height:`${Math.max(4, Math.min(100, engine.score))}%`}} /></div>
                <span>{engine.name}</span><b>{engine.score.toFixed(0)}</b>
              </div>)}
            </div>
          </section>

          <aside className="ws-decision-dock">
            <div className="ws-decision-top"><span>{t.ccCurrentDecision}</span><div className={`ws-decision-value ${decisionClass}`}>{data.decision}</div><div className="ws-decision-price">{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</div></div>
            <div className="ws-decision-facts">
              <div className="ws-decision-fact"><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b></div>
              <div className="ws-decision-fact"><span>{t.colRisk}</span><b>{data.risk.toFixed(1)}%</b></div>
              <div className="ws-decision-fact"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
              <div className="ws-decision-fact"><span>{t.colRegime}</span><b>{data.regime}</b></div>
            </div>
            <div className="ws-decision-reason">{data.reasons[0] ?? t.ccNoExplanation}</div>
          </aside>
        </div>

        <div className="ws-engine-tape">
          {data.engines.map(engine => <div className="ws-engine-tile" key={engine.name}><span>{engine.name}</span><strong>{engine.score.toFixed(0)}</strong><div className="ws-engine-track"><i style={{width:`${Math.max(0,Math.min(100,engine.score))}%`}} /></div><small>{engine.state} · w {engine.weight.toFixed(2)}</small></div>)}
        </div>
      </main>

      <aside className="ws-pane ws-right">
        <section className="ws-side-section">
          <div className="ws-pane-head"><strong>{t.ccRiskGuardianLabel}</strong><span>{blocked ? t.ccBlocked : t.ccActive}</span></div>
          <div className="ws-side-body"><div className="ws-risk-number"><strong>{data.risk.toFixed(1)}%</strong><span>{blocked ? t.itTradeBlocked : t.itDecisionActive}</span></div><div className="ws-risk-track"><i style={{width:`${Math.max(0,Math.min(100,data.risk))}%`}} /></div><div className="ws-side-copy">{data.reasons[0] ?? t.ccNoExplanation}</div></div>
        </section>

        <section className="ws-side-section">
          <div className="ws-pane-head"><strong>{t.pcModelPerformance}</strong><span>{data.symbol}</span></div>
          <div className="ws-perf-table">
            <div className="ws-perf-row"><span>{t.ccMetaConfidence}</span><b>{data.confidence.toFixed(1)}%</b></div>
            <div className="ws-perf-row"><span>{t.pcWinRate}</span><b>{showHistorical && realized ? `${realized.winRate.toFixed(1)}%` : '—'}</b></div>
            <div className="ws-perf-row"><span>{t.pcRealizedSamples}</span><b>{showHistorical && realized ? realized.samples : '—'}</b></div>
            <div className="ws-perf-row"><span>{t.ccQualityLabel}</span><b>{data.quality.toFixed(1)}%</b></div>
          </div>
        </section>

        <section className="ws-side-section ws-reasons">
          <div className="ws-pane-head"><strong>{t.ccLatestIntelligence}</strong><span>{data.reasons.length}</span></div>
          {data.reasons.length ? data.reasons.map((reason,index)=><div className="ws-reason" key={index}><i>{String(index+1).padStart(2,'0')}</i><span>{reason}</span></div>) : <div className="ws-reason"><i>01</i><span>{t.ccNoExplanation}</span></div>}
        </section>
      </aside>

      <section className="ws-pane ws-bottom">
        <div className="ws-history">
          <div className="ws-pane-head"><strong>{t.navHistory}</strong><span>{history.data?.length ?? 0}</span></div>
          <div className="ws-history-body">
            {history.data?.map(item => <div className="ws-history-row" key={item.id}><b>{item.symbol}</b><span>{item.decision} · {item.confidence.toFixed(0)}%</span><span className={item.realizedReturn === undefined ? '' : item.realizedReturn >= 0 ? 'positive' : 'negative'}>{item.realizedReturn !== undefined ? `${item.realizedReturn >= 0 ? '+' : ''}${item.realizedReturn.toFixed(2)}%` : '—'}</span><span>{item.horizon}</span><small>{item.outcome ?? t.historyPending}</small></div>)}
            {!history.loading && (!history.data || history.data.length === 0) && <div className="ws-side-copy" style={{padding:10}}>{history.error || t.historyEmpty}</div>}
          </div>
        </div>
        <div className="ws-system">
          <div className="ws-pane-head"><strong>{t.navHealth}</strong><span>{health.data?.length ?? 0}</span></div>
          <div className="ws-system-body">
            {health.data?.slice(0,6).map(item => <div className="ws-system-metric" key={item.name}><span>{item.name}</span><b className={healthStatusClass(item.status)}>{item.status.toUpperCase()}</b></div>)}
          </div>
        </div>
      </section>
    </div>
  </div>;
}
