//! Wire types for Binance WebSocket and REST payloads, and their
//! conversion into `shared` domain types.

use serde::Deserialize;
use shared::{Candle, FundingRate, OrderBookLevel, OrderBookSnapshot};

/// Envelope used by Binance's combined stream endpoint
/// (`/stream?streams=a/b/c`): `{"stream": "<name>", "data": <payload>}`.
#[derive(Debug, Deserialize)]
pub struct CombinedStreamEnvelope<T> {
    pub stream: String,
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct KlineEvent {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: KlinePayload,
}

#[derive(Debug, Deserialize)]
pub struct KlinePayload {
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "T")]
    pub close_time: i64,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
}

impl KlineEvent {
    pub fn into_candle(self) -> Result<Candle, std::num::ParseFloatError> {
        Ok(Candle {
            symbol: self.symbol,
            interval: self.kline.interval,
            open_time: self.kline.open_time,
            close_time: self.kline.close_time,
            open: self.kline.open.parse()?,
            high: self.kline.high.parse()?,
            low: self.kline.low.parse()?,
            close: self.kline.close.parse()?,
            volume: self.kline.volume.parse()?,
        })
    }
}

/// Spot partial book depth payload (`<symbol>@depth20`). Binance does not
/// embed the symbol or a timestamp in this payload, so both are supplied
/// by the caller from the stream name and the local clock.
#[derive(Debug, Deserialize)]
pub struct DepthPayload {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

impl DepthPayload {
    pub fn into_snapshot(
        self,
        symbol: String,
        timestamp: i64,
    ) -> Result<OrderBookSnapshot, std::num::ParseFloatError> {
        let to_levels = |raw: Vec<[String; 2]>| -> Result<Vec<OrderBookLevel>, std::num::ParseFloatError> {
            raw.into_iter()
                .map(|[price, quantity]| {
                    Ok(OrderBookLevel {
                        price: price.parse()?,
                        quantity: quantity.parse()?,
                    })
                })
                .collect()
        };
        Ok(OrderBookSnapshot {
            symbol,
            timestamp,
            bids: to_levels(self.bids)?,
            asks: to_levels(self.asks)?,
        })
    }
}

/// Futures `markPriceUpdate` event, which carries the current funding
/// rate (`r`). Funding rate is a perpetual-futures concept and does not
/// exist on the Binance spot market, so this is read from the futures
/// WebSocket rather than the spot one.
#[derive(Debug, Deserialize)]
pub struct MarkPriceEvent {
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "r")]
    pub funding_rate: String,
}

impl MarkPriceEvent {
    pub fn into_funding_rate(self) -> Result<FundingRate, std::num::ParseFloatError> {
        Ok(FundingRate {
            symbol: self.symbol,
            timestamp: self.event_time,
            rate: self.funding_rate.parse()?,
        })
    }
}

/// A single entry from the REST funding rate history endpoint
/// (`GET /fapi/v1/fundingRate`).
#[derive(Debug, Deserialize)]
pub struct FundingRateHistoryEntry {
    pub symbol: String,
    #[serde(rename = "fundingTime")]
    pub funding_time: i64,
    #[serde(rename = "fundingRate")]
    pub funding_rate: String,
}

impl FundingRateHistoryEntry {
    pub fn into_funding_rate(self) -> Result<FundingRate, std::num::ParseFloatError> {
        Ok(FundingRate {
            symbol: self.symbol,
            timestamp: self.funding_time,
            rate: self.funding_rate.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kline_event() {
        let raw = r#"{
            "e": "kline", "E": 123456789, "s": "BTCUSDT",
            "k": {
                "t": 123400000, "T": 123460000, "s": "BTCUSDT", "i": "1m",
                "o": "0.0010", "c": "0.0020", "h": "0.0025", "l": "0.0015",
                "v": "1000", "x": false
            }
        }"#;
        let event: KlineEvent = serde_json::from_str(raw).unwrap();
        let candle = event.into_candle().unwrap();
        assert_eq!(candle.symbol, "BTCUSDT");
        assert_eq!(candle.interval, "1m");
        assert_eq!(candle.open, 0.0010);
        assert_eq!(candle.close, 0.0020);
    }

    #[test]
    fn parses_depth_payload() {
        let raw = r#"{
            "lastUpdateId": 160,
            "bids": [["0.0024", "10"]],
            "asks": [["0.0026", "100"]]
        }"#;
        let payload: DepthPayload = serde_json::from_str(raw).unwrap();
        let snapshot = payload.into_snapshot("BTCUSDT".to_string(), 42).unwrap();
        assert_eq!(snapshot.symbol, "BTCUSDT");
        assert_eq!(snapshot.timestamp, 42);
        assert_eq!(snapshot.bids[0].price, 0.0024);
        assert_eq!(snapshot.asks[0].quantity, 100.0);
    }

    #[test]
    fn parses_mark_price_event() {
        let raw = r#"{
            "e": "markPriceUpdate", "E": 1562305380000, "s": "BTCUSDT",
            "p": "11185.87786614", "P": "11215.13792037",
            "i": "11189.14071228", "r": "0.00030000", "T": 1562306400000
        }"#;
        let event: MarkPriceEvent = serde_json::from_str(raw).unwrap();
        let funding = event.into_funding_rate().unwrap();
        assert_eq!(funding.symbol, "BTCUSDT");
        assert_eq!(funding.rate, 0.0003);
    }

    #[test]
    fn parses_combined_stream_envelope() {
        let raw = r#"{"stream":"btcusdt@depth20","data":{"lastUpdateId":1,"bids":[],"asks":[]}}"#;
        let envelope: CombinedStreamEnvelope<DepthPayload> = serde_json::from_str(raw).unwrap();
        assert_eq!(envelope.stream, "btcusdt@depth20");
    }
}
