//! Deterministic microstructure and derivatives analysis.

use shared::{FundingRate, OpenInterest, OrderBookSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DominantSide { Buy, Sell, Neutral }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidityResult {
    pub order_flow_score: f64,
    pub liquidity_score: f64,
    pub buy_pressure: f64,
    pub sell_pressure: f64,
    pub spread_bps: f64,
    pub dominant_side: DominantSide,
    pub liquidity_risk: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivativesPressure { LongBuildUp, ShortBuildUp, ShortCovering, LongUnwinding, Neutral, InsufficientData }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivativesResult {
    pub oi_change_pct: f64,
    pub funding_rate: f64,
    pub leverage_score: f64,
    pub derivatives_score: f64,
    pub pressure: DerivativesPressure,
}

pub fn liquidity_order_flow(book: &OrderBookSnapshot) -> LiquidityResult {
    let bid_qty: f64 = book.bids.iter().map(|x| x.quantity.max(0.0)).sum();
    let ask_qty: f64 = book.asks.iter().map(|x| x.quantity.max(0.0)).sum();
    let total = bid_qty + ask_qty;
    let imbalance = if total > 0.0 { (bid_qty - ask_qty) / total } else { 0.0 };
    let best_bid = book.bids.iter().map(|x| x.price).fold(f64::NEG_INFINITY, f64::max);
    let best_ask = book.asks.iter().map(|x| x.price).fold(f64::INFINITY, f64::min);
    let mid = (best_bid + best_ask) / 2.0;
    let spread_bps = if best_bid.is_finite() && best_ask.is_finite() && mid > 0.0 { ((best_ask - best_bid).max(0.0) / mid) * 10_000.0 } else { 10_000.0 };
    let buy_pressure = ((imbalance + 1.0) * 50.0).clamp(0.0, 100.0);
    let sell_pressure = 100.0 - buy_pressure;
    let depth_score = (total.ln_1p() * 12.0).clamp(0.0, 100.0);
    let spread_penalty = (spread_bps * 2.0).clamp(0.0, 100.0);
    let liquidity_score = (depth_score - spread_penalty * 0.5).clamp(0.0, 100.0);
    let liquidity_risk = (100.0 - liquidity_score).clamp(0.0, 100.0);
    let dominant_side = if imbalance > 0.1 { DominantSide::Buy } else if imbalance < -0.1 { DominantSide::Sell } else { DominantSide::Neutral };
    LiquidityResult { order_flow_score: (imbalance * 100.0).clamp(-100.0, 100.0), liquidity_score, buy_pressure, sell_pressure, spread_bps, dominant_side, liquidity_risk }
}

pub fn derivatives(previous: Option<&OpenInterest>, current: Option<&OpenInterest>, funding: &FundingRate, price_change_pct: f64) -> DerivativesResult {
    let (Some(prev), Some(cur)) = (previous, current) else {
        return DerivativesResult { oi_change_pct: 0.0, funding_rate: funding.rate, leverage_score: 0.0, derivatives_score: 0.0, pressure: DerivativesPressure::InsufficientData };
    };
    let oi_change_pct = if prev.value > 0.0 { ((cur.value / prev.value) - 1.0) * 100.0 } else { 0.0 };
    let pressure = match (price_change_pct >= 0.0, oi_change_pct >= 0.0) {
        (true, true) => DerivativesPressure::LongBuildUp,
        (false, true) => DerivativesPressure::ShortBuildUp,
        (true, false) => DerivativesPressure::ShortCovering,
        (false, false) => DerivativesPressure::LongUnwinding,
    };
    let funding_component = (funding.rate * 100_000.0).clamp(-50.0, 50.0);
    let oi_component = (oi_change_pct * 10.0).clamp(-50.0, 50.0);
    let direction = if price_change_pct >= 0.0 { 1.0 } else { -1.0 };
    let derivatives_score = (50.0 + direction * oi_component + funding_component).clamp(0.0, 100.0);
    let leverage_score = ((oi_change_pct.abs() * 15.0) + (funding.rate.abs() * 100_000.0)).clamp(0.0, 100.0);
    DerivativesResult { oi_change_pct, funding_rate: funding.rate, leverage_score, derivatives_score, pressure }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::OrderBookLevel;

    #[test]
    fn detects_buy_dominance() {
        let book = OrderBookSnapshot { symbol: "BTCUSDT".into(), timestamp: 1, bids: vec![OrderBookLevel { price: 100.0, quantity: 30.0 }], asks: vec![OrderBookLevel { price: 101.0, quantity: 10.0 }] };
        let result = liquidity_order_flow(&book);
        assert_eq!(result.dominant_side, DominantSide::Buy);
        assert!(result.order_flow_score > 0.0);
    }

    #[test]
    fn empty_book_is_high_risk() {
        let book = OrderBookSnapshot { symbol: "BTCUSDT".into(), timestamp: 1, bids: vec![], asks: vec![] };
        assert!(liquidity_order_flow(&book).liquidity_risk > 90.0);
    }

    #[test]
    fn price_and_oi_rise_is_long_build_up() {
        let p = OpenInterest { symbol: "BTCUSDT".into(), timestamp: 1, value: 100.0 };
        let c = OpenInterest { symbol: "BTCUSDT".into(), timestamp: 2, value: 110.0 };
        let f = FundingRate { symbol: "BTCUSDT".into(), timestamp: 2, rate: 0.0001 };
        assert_eq!(derivatives(Some(&p), Some(&c), &f, 2.0).pressure, DerivativesPressure::LongBuildUp);
    }
}
