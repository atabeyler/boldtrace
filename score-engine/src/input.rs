//! Input data for a single `calculate` call.

use shared::{Candle, FundingRate, OrderBookSnapshot};

/// Everything `calculate` needs to score one symbol at one point in time.
/// `candles` must be sorted oldest-first and share the same symbol and
/// interval as each other.
#[derive(Debug, Clone)]
pub struct ScoreInput {
    pub candles: Vec<Candle>,
    pub order_book: OrderBookSnapshot,
    pub funding_rate: FundingRate,
}
