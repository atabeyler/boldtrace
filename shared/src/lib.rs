//! Shared data types used across the Boldtrace workspace.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle { pub symbol:String,pub interval:String,pub open_time:i64,pub close_time:i64,pub open:f64,pub high:f64,pub low:f64,pub close:f64,pub volume:f64 }
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct OrderBookLevel{pub price:f64,pub quantity:f64}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct OrderBookSnapshot{pub symbol:String,pub timestamp:i64,pub bids:Vec<OrderBookLevel>,pub asks:Vec<OrderBookLevel>}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct FundingRate{pub symbol:String,pub timestamp:i64,pub rate:f64}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct OpenInterest{pub symbol:String,pub timestamp:i64,pub value:f64}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct Score{pub symbol:String,pub timestamp:i64,pub value:f64,pub volume_anomaly:f64,pub funding_extreme:f64,pub order_book_imbalance:f64,pub rsi_divergence:f64}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct Signal{pub symbol:String,pub timestamp:i64,pub score:f64,pub threshold:f64}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct User{pub telegram_id:i64,pub language:String,pub consent_given_at:Option<i64>,pub consent_terms_version:Option<String>}
#[derive(Debug, Clone, Serialize, Deserialize)]pub struct Session{pub telegram_id:i64,pub started_at:i64}
/// Transport-neutral snapshot published by the intelligence runtime for product surfaces.
#[derive(Debug,Clone,Serialize,Deserialize)]pub struct LiveIntelligence{pub symbol:String,pub timestamp:i64,pub price:f64,pub score:Score,pub decision:String,pub confidence:f64,pub risk:f64,pub data_quality:f64,pub agreement:f64,pub regime:String,pub volume_weight:f64,pub funding_weight:f64,pub order_book_weight:f64,pub rsi_weight:f64,pub reasons:Vec<String>,pub warnings:Vec<String>}
