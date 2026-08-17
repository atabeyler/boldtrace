//! Order book imbalance signal: how skewed the visible bid/ask volume is,
//! expressed as a 0-100 score.

use shared::OrderBookSnapshot;

/// Scores how imbalanced `order_book`'s bid and ask volume is, regardless
/// of direction. `0.0` means bid and ask volume are equal; `100.0` means
/// all visible volume sits on one side. Returns `0.0` when the book is
/// empty on both sides.
pub fn order_book_imbalance(order_book: &OrderBookSnapshot) -> f64 {
    let bid_volume: f64 = order_book.bids.iter().map(|level| level.quantity).sum();
    let ask_volume: f64 = order_book.asks.iter().map(|level| level.quantity).sum();
    let total = bid_volume + ask_volume;
    if total == 0.0 {
        return 0.0;
    }
    (((bid_volume - ask_volume) / total).abs() * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::OrderBookLevel;

    fn book(bids: Vec<f64>, asks: Vec<f64>) -> OrderBookSnapshot {
        OrderBookSnapshot {
            symbol: "BTCUSDT".to_string(),
            timestamp: 0,
            bids: bids
                .into_iter()
                .map(|quantity| OrderBookLevel { price: 1.0, quantity })
                .collect(),
            asks: asks
                .into_iter()
                .map(|quantity| OrderBookLevel { price: 1.0, quantity })
                .collect(),
        }
    }

    #[test]
    fn empty_book_scores_zero() {
        assert_eq!(order_book_imbalance(&book(vec![], vec![])), 0.0);
    }

    #[test]
    fn balanced_book_scores_zero() {
        assert_eq!(order_book_imbalance(&book(vec![10.0], vec![10.0])), 0.0);
    }

    #[test]
    fn one_sided_book_scores_full() {
        assert_eq!(order_book_imbalance(&book(vec![10.0], vec![])), 100.0);
    }

    #[test]
    fn skew_is_direction_agnostic() {
        let bid_heavy = order_book_imbalance(&book(vec![30.0], vec![10.0]));
        let ask_heavy = order_book_imbalance(&book(vec![10.0], vec![30.0]));
        assert_eq!(bid_heavy, ask_heavy);
        assert_eq!(bid_heavy, 50.0);
    }
}
