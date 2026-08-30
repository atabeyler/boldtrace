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
  const decisionClass = data.decision.toLowerCase().replaceAll(' ', '-');
  const riskTitle = blocked ? t.itTradeBlocked : stale ? t.itDataStale : error ? t.itConnectionDegraded : t.itDecisionActive;
  const riskDetail = blocked ? (data.reasons[0] ?? t.itNoExplanation) : stale ? `${t.itUpdated}: ${Math.round(data.freshnessMs / 1000)}s` : error ? t.itRefreshFailed : `${t.itDataFresh} · ${t.colRisk} ${data.risk.toFixed(1)}%`;

  return <div className="page">
    <div className="page-head">
      <div><span className="eyebrow">{t.itEyebrowPrefix} {data.symbol}</span><h1>{t.itTitle}</h1><p>{t.itSub}</p></div>
      <button className={`system-pill ${error ? 'status-pill--bad' : stale ? 'status-pill--warn' : 'status-pill--ok'}`} onClick={refresh} aria-label={t.retryConnection}><i />{error ? t.itConnectionDegraded : stale ? t.itDataStale : t.itDataFresh}</button>
    </div>

    <div className="command-terminal">
      <div className="command-main">
        <section className="command-market-board">
          <div className="command-market-strip">
            <div><strong>{data.symbol}</strong><small>{t.itMarketState}</small></div>
            <div className="command-strip-value"><b>{data.price > 0 ? `$${data.price.toLocaleString()}` : '—'}</b> · {t.colRegime}: {data.regime}</div>
            <div className="command-strip-value">{t.itUpdated}: <b>{Math.round(data.freshnessMs / 1000)}s</b></div>
            <div className="command-strip-value">{t.ccQualityLabel}: <b>{data.quality.toFixed(1)}%</b></div>
          </div>

          <div className="command-core">
            <div className="command-decision-zone">
              <div>
                <div className="command-decision-kicker"><i/><span>{t.itMetaDecision}</span></div>
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
              <div className="command-spectrum-head"><div><span className="eyebrow">{t.itEngineEvidence}</span><h2>{t.itConsensusMatrix}</h2></div><small>{data.engines.length} {t.ccLiveEngines}<br/>{t.colRegime}: {data.regime}</small></div>
              <div className="command-spectrum" aria-label={t.itConsensusMatrix}>{data.engines.map(engine => <div className="command-spectrum-col" key={engine.name} title={`${engine.name}: ${engine.score.toFixed(1)}`}><div className="command-spectrum-bar" style={{height:`${Math.max(5,Math.min(100,engine.score))}%`}}/><span>{engine.name}</span><b>{engine.score.toFixed(0)}</b></div>)}</div>
              <div className="command-spectrum-footer"><span>{t.itRuntimeSnapshot}</span><b>{stale ? t.itDataStale : t.itDataFresh}</b></div>
            </div>
          </div>
        </section>

        <div className="command-analysis-grid">
          <section className="panel command-analysis-card">
            <div className="panel-head"><div><span className="eyebrow">{t.itExplainability}</span><h2>{t.itWhyDecision}</h2></div><b>{data.symbol}</b></div>
            <div className="command-reasons">{data.reasons.length ? data.reasons.map((reason,index)=><div className="command-reason" key={index}><span className="command-reason-index">{String(index+1).padStart(2,'0')}</span><span>{reason}</span></div>) : <div className="command-reason"><span className="command-reason-index">01</span><span>{t.itNoExplanation}</span></div>}</div>
          </section>

          <section className="panel command-engine-ledger">
            <span className="eyebrow">{t.itEngineEvidence}</span><h2>{data.symbol}</h2>
            <div className="command-engine-ledger-list">{data.engines.map(engine=><div className="command-engine-ledger-row" key={engine.name}><strong>{engine.name}</strong><span>{engine.state}</span><span>w {engine.weight.toFixed(2)}</span><b>{engine.score.toFixed(0)}</b></div>)}</div>
          </section>
        </div>
      </div>

      <aside className="command-rail">
        <section className="command-risk-panel">
          <div className="command-risk-header"><div><span className="eyebrow">{t.itRiskGuardian}</span><h2>{riskTitle}</h2></div><span className={`command-risk-state ${blocked ? 'blocked' : ''}`}>{blocked ? t.ccBlocked : t.ccActive}</span></div>
          <div className="command-risk-meter"><div className="command-risk-meter-head"><strong>{data.risk.toFixed(1)}%</strong><span>{t.colRisk}</span></div><div className="command-risk-track"><i style={{width:`${Math.max(0,Math.min(100,data.risk))}%`}}/></div></div>
          <div className="command-risk-copy">{riskDetail}</div>
        </section>

        <section className="panel command-performance-panel">
          <span className="eyebrow">{t.itRuntimeSnapshot}</span><h2>{data.symbol}</h2>
          <div className="command-performance-grid">
            <div className="command-performance-cell"><span>{t.ccMetaConfidence}</span><strong>{data.confidence.toFixed(1)}%</strong><small>{t.ccConfidenceSub}</small></div>
            <div className="command-performance-cell"><span>{t.ccQualityLabel}</span><strong>{data.quality.toFixed(1)}%</strong><small>{t.colRegime}: {data.regime}</small></div>
          </div>
        </section>
      </aside>
    </div>
  </div>;
}

function State({ t, title, detail, action }: { t: Copy; title: string; detail?: string; action?: () => void }) {
  return <div className="page"><section className="panel empty-state" role={detail ? 'alert' : 'status'}><span className="eyebrow">{t.itLiveEyebrow}</span><h2>{title}</h2>{detail && <p>{detail}</p>}{action && <button onClick={action}>{t.retryConnection}</button>}</section></div>;
}
