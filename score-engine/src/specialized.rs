//! Specialized deterministic engines that complement the composite score.
use crate::{Decision, DerivativesResult, LiquidityResult, MarketRegime};
use shared::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepSide { Highs, Lows, None }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepResult { pub score: f64, pub side: SweepSide, pub rejection: f64 }

pub fn liquidity_sweep(c: &[Candle]) -> SweepResult {
    if c.len() < 3 { return SweepResult { score: 0.0, side: SweepSide::None, rejection: 0.0 }; }
    let last = &c[c.len() - 1];
    let prior = &c[..c.len() - 1];
    let high = prior.iter().map(|x| x.high).fold(f64::NEG_INFINITY, f64::max);
    let low = prior.iter().map(|x| x.low).fold(f64::INFINITY, f64::min);
    let range = (last.high - last.low).max(f64::EPSILON);
    if last.high > high && last.close < high {
        let rejection = ((last.high - last.close) / range * 100.0).clamp(0.0, 100.0);
        SweepResult { score: rejection, side: SweepSide::Highs, rejection }
    } else if last.low < low && last.close > low {
        let rejection = ((last.close - last.low) / range * 100.0).clamp(0.0, 100.0);
        SweepResult { score: rejection, side: SweepSide::Lows, rejection }
    } else {
        SweepResult { score: 0.0, side: SweepSide::None, rejection: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShockResult { pub score: f64, pub volume_shock: f64, pub range_shock: f64 }
pub fn market_shock(c: &[Candle]) -> ShockResult {
    if c.len() < 6 { return ShockResult { score: 0.0, volume_shock: 0.0, range_shock: 0.0 }; }
    let last = &c[c.len() - 1]; let base = &c[c.len() - 6..c.len() - 1];
    let av = base.iter().map(|x| x.volume).sum::<f64>() / base.len() as f64;
    let ar = base.iter().map(|x| (x.high - x.low).abs()).sum::<f64>() / base.len() as f64;
    let volume_shock = if av > 0.0 { ((last.volume / av - 1.0) * 50.0).clamp(0.0, 100.0) } else { 0.0 };
    let range = (last.high - last.low).abs();
    let range_shock = if ar > 0.0 { ((range / ar - 1.0) * 50.0).clamp(0.0, 100.0) } else { 0.0 };
    ShockResult { score: (volume_shock * 0.55 + range_shock * 0.45).clamp(0.0, 100.0), volume_shock, range_shock }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivativesStress { pub score: f64, pub squeeze_risk: f64 }
pub fn derivatives_stress(d: Option<&DerivativesResult>) -> DerivativesStress {
    let Some(d) = d else { return DerivativesStress { score: 0.0, squeeze_risk: 0.0 }; };
    let funding = (d.funding_rate.abs() * 100_000.0).clamp(0.0, 100.0);
    let oi = (d.oi_change_pct.abs() * 12.0).clamp(0.0, 100.0);
    let score = (d.leverage_score * 0.45 + funding * 0.30 + oi * 0.25).clamp(0.0, 100.0);
    DerivativesStress { score, squeeze_risk: score }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConflictResult { pub score: f64, pub force_no_trade: bool }
pub fn signal_conflict(decision: Decision, composite: f64, derivatives: Option<f64>, liquidity: Option<&LiquidityResult>) -> ConflictResult {
    let mut conflict: f64 = 0.0;
    let dir = match decision { Decision::StrongLong | Decision::Long => 1.0, Decision::StrongShort | Decision::Short => -1.0, _ => 0.0 };
    if let Some(d) = derivatives {
        let dd = (d - 50.0).signum();
        if dir != 0.0 && dd != 0.0 && dir != dd { conflict += 45.0; }
    }
    if let Some(l) = liquidity {
        let ld = l.order_flow_score.signum();
        if dir != 0.0 && ld != 0.0 && dir != ld { conflict += 35.0; }
        if l.liquidity_risk > 75.0 { conflict += 20.0; }
    }
    if (composite - 50.0).abs() < 8.0 { conflict += 10.0; }
    let score = conflict.clamp(0.0, 100.0);
    ConflictResult { score, force_no_trade: score >= 70.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeTransition { pub score: f64, pub transitioning: bool }
pub fn regime_transition(previous: Option<MarketRegime>, current: MarketRegime, shock: f64) -> RegimeTransition {
    let changed = previous.is_some_and(|p| p != current);
    let base = if changed { 70.0 } else { 0.0 };
    let score = (base + shock * 0.30).clamp(0.0, 100.0);
    RegimeTransition { score, transitioning: score >= 60.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossMarketResult { pub score: f64, pub divergence: f64 }
pub fn cross_market(primary_returns: &[f64], reference_returns: &[f64]) -> CrossMarketResult {
    let n = primary_returns.len().min(reference_returns.len());
    if n < 3 { return CrossMarketResult { score: 0.0, divergence: 0.0 }; }
    let p = primary_returns[n - 3..n].iter().sum::<f64>();
    let r = reference_returns[reference_returns.len() - 3..].iter().sum::<f64>();
    let divergence = (p - r).abs();
    CrossMarketResult { score: (divergence * 20.0).clamp(0.0, 100.0), divergence }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn c(h: f64, l: f64, close: f64, v: f64) -> Candle { Candle { symbol: "BTCUSDT".into(), interval: "1m".into(), open_time: 0, close_time: 1, open: close, high: h, low: l, close, volume: v } }
    #[test] fn detects_high_sweep() { let x = vec![c(100.,90.,95.,10.),c(101.,91.,96.,10.),c(110.,94.,99.,30.)]; assert_eq!(liquidity_sweep(&x).side, SweepSide::Highs); }
    #[test] fn shock_rises_on_expansion() { let mut x=vec![c(101.,99.,100.,10.);5]; x.push(c(120.,80.,110.,100.)); assert!(market_shock(&x).score > 50.0); }
    #[test] fn conflict_can_force_no_trade() { let l=LiquidityResult{order_flow_score:-80.0,liquidity_score:80.0,buy_pressure:10.0,sell_pressure:90.0,spread_bps:1.0,dominant_side:crate::DominantSide::Sell,liquidity_risk:20.0}; let r=signal_conflict(Decision::Long,80.0,Some(20.0),Some(&l)); assert!(r.force_no_trade); }
}
