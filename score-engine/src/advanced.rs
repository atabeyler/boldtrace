//! Advanced deterministic market-intelligence engines.
//!
//! These calculations deliberately perform no I/O so the same logic can be
//! reused by the live pipeline and historical backtests without look-ahead.

use shared::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketRegime {
    StrongBull,
    Bull,
    Sideways,
    Bear,
    StrongBear,
    HighVolatility,
    LowVolatility,
    Compression,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeResult {
    pub regime: MarketRegime,
    pub confidence: f64,
    pub trend_strength: f64,
    pub momentum: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityState {
    Normal,
    Compression,
    Expanding,
    Extreme,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilityResult {
    pub score: f64,
    pub compression_score: f64,
    pub expansion_score: f64,
    pub state: VolatilityState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureState {
    HigherHighHigherLow,
    LowerHighLowerLow,
    Range,
    Breakout,
    Breakdown,
    InsufficientData,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureResult {
    pub structure: StructureState,
    pub strength: f64,
}

fn returns(candles: &[Candle]) -> Vec<f64> {
    candles
        .windows(2)
        .filter_map(|w| {
            let previous = w[0].close;
            (previous > 0.0).then_some((w[1].close / previous) - 1.0)
        })
        .collect()
}

fn realized_volatility(candles: &[Candle]) -> f64 {
    let r = returns(candles);
    if r.len() < 2 {
        return 0.0;
    }
    let mean = r.iter().sum::<f64>() / r.len() as f64;
    let variance = r.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / r.len() as f64;
    variance.sqrt()
}

fn average_true_range_pct(candles: &[Candle]) -> f64 {
    if candles.is_empty() {
        return 0.0;
    }
    let start = candles.len().saturating_sub(14);
    let slice = &candles[start..];
    let sum = slice
        .iter()
        .filter_map(|c| (c.close > 0.0).then_some((c.high - c.low).abs() / c.close))
        .sum::<f64>();
    sum / slice.len() as f64
}

pub fn volatility(candles: &[Candle]) -> VolatilityResult {
    if candles.len() < 6 {
        return VolatilityResult { score: 0.0, compression_score: 0.0, expansion_score: 0.0, state: VolatilityState::Normal };
    }
    let split = candles.len() / 2;
    let old = realized_volatility(&candles[..split]);
    let recent = realized_volatility(&candles[split..]);
    let ratio = if old > f64::EPSILON { recent / old } else { 1.0 };
    let atr = average_true_range_pct(candles);
    let score = ((recent * 5_000.0) + (atr * 2_500.0)).clamp(0.0, 100.0);
    let compression_score = ((1.0 - ratio).max(0.0) * 100.0).clamp(0.0, 100.0);
    let expansion_score = ((ratio - 1.0).max(0.0) * 100.0).clamp(0.0, 100.0);
    let state = if score >= 85.0 { VolatilityState::Extreme } else if ratio >= 1.5 { VolatilityState::Expanding } else if ratio <= 0.65 { VolatilityState::Compression } else { VolatilityState::Normal };
    VolatilityResult { score, compression_score, expansion_score, state }
}

pub fn market_regime(candles: &[Candle]) -> RegimeResult {
    if candles.len() < 10 {
        return RegimeResult { regime: MarketRegime::Sideways, confidence: 0.0, trend_strength: 0.0, momentum: 0.0, volatility: 0.0 };
    }
    let first = candles[candles.len() - 10].close;
    let last = candles.last().map(|c| c.close).unwrap_or(first);
    let momentum = if first > 0.0 { ((last / first) - 1.0) * 100.0 } else { 0.0 };
    let trend_strength = (momentum.abs() * 20.0).clamp(0.0, 100.0);
    let vol = volatility(candles);
    let regime = if vol.state == VolatilityState::Compression {
        MarketRegime::Compression
    } else if vol.state == VolatilityState::Extreme {
        MarketRegime::HighVolatility
    } else if vol.score < 5.0 {
        MarketRegime::LowVolatility
    } else if momentum >= 5.0 {
        MarketRegime::StrongBull
    } else if momentum >= 1.0 {
        MarketRegime::Bull
    } else if momentum <= -5.0 {
        MarketRegime::StrongBear
    } else if momentum <= -1.0 {
        MarketRegime::Bear
    } else {
        MarketRegime::Sideways
    };
    RegimeResult { regime, confidence: trend_strength.max(vol.compression_score).clamp(0.0, 100.0), trend_strength, momentum, volatility: vol.score }
}

pub fn market_structure(candles: &[Candle]) -> StructureResult {
    if candles.len() < 6 {
        return StructureResult { structure: StructureState::InsufficientData, strength: 0.0 };
    }
    let n = candles.len();
    let a = &candles[n - 6..n - 3];
    let b = &candles[n - 3..];
    let a_high = a.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
    let a_low = a.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let b_high = b.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
    let b_low = b.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let close = candles[n - 1].close;
    let structure = if close > a_high { StructureState::Breakout } else if close < a_low { StructureState::Breakdown } else if b_high > a_high && b_low > a_low { StructureState::HigherHighHigherLow } else if b_high < a_high && b_low < a_low { StructureState::LowerHighLowerLow } else { StructureState::Range };
    let base = (a_high - a_low).abs().max(f64::EPSILON);
    let strength = (((b_high - a_high).abs() + (b_low - a_low).abs()) / base * 50.0).clamp(0.0, 100.0);
    StructureResult { structure, strength }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles(step: f64) -> Vec<Candle> {
        (0..30).map(|i| {
            let close = 100.0 + i as f64 * step;
            Candle { symbol: "BTCUSDT".into(), interval: "5m".into(), open_time: i * 300_000, close_time: (i + 1) * 300_000, open: close - step, high: close + 0.5, low: close - 0.5, close, volume: 100.0 }
        }).collect()
    }

    #[test]
    fn rising_market_is_bullish() {
        assert!(matches!(market_regime(&candles(1.0)).regime, MarketRegime::Bull | MarketRegime::StrongBull));
    }

    #[test]
    fn falling_market_is_bearish() {
        assert!(matches!(market_regime(&candles(-1.0)).regime, MarketRegime::Bear | MarketRegime::StrongBear));
    }

    #[test]
    fn calculations_are_bounded() {
        let c = candles(0.2);
        let v = volatility(&c);
        assert!((0.0..=100.0).contains(&v.score));
        assert!((0.0..=100.0).contains(&market_structure(&c).strength));
    }
}
