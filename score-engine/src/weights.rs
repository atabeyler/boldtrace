//! Configurable and adaptive weights for the composite score.
/// Below this many realized-outcome samples, adaptation is skipped and the
/// base weights are used unchanged. Exposed so callers (e.g. the product
/// API's Learning Center) can show the real guardrail instead of a
/// restated magic number.
pub const MIN_SAMPLES_FOR_ADAPTATION: usize = 30;
/// Bounds an adapted weight is clamped into, so no single signal can ever
/// dominate or be silenced entirely.
pub const WEIGHT_MIN: f64 = 0.10;
pub const WEIGHT_MAX: f64 = 0.45;
#[derive(Debug,Clone,Copy,PartialEq)]pub struct ScoreWeights{pub volume_anomaly:f64,pub funding_extreme:f64,pub order_book_imbalance:f64,pub rsi_divergence:f64}
#[derive(Debug,Clone,Copy,PartialEq)]pub struct SignalReliability{pub volume:f64,pub funding:f64,pub order_book:f64,pub rsi:f64,pub samples:usize}
impl SignalReliability{pub fn neutral()->Self{Self{volume:50.0,funding:50.0,order_book:50.0,rsi:50.0,samples:0}}}
impl ScoreWeights{pub fn from_env()->Self{let d=Self::default();Self{volume_anomaly:env_weight("SCORE_WEIGHT_VOLUME_ANOMALY",d.volume_anomaly),funding_extreme:env_weight("SCORE_WEIGHT_FUNDING_EXTREME",d.funding_extreme),order_book_imbalance:env_weight("SCORE_WEIGHT_ORDER_BOOK_IMBALANCE",d.order_book_imbalance),rsi_divergence:env_weight("SCORE_WEIGHT_RSI_DIVERGENCE",d.rsi_divergence)}.normalized()}
pub fn normalized(self)->Self{let sum=self.volume_anomaly+self.funding_extreme+self.order_book_imbalance+self.rsi_divergence;if sum<=0.0{return Self::default();}Self{volume_anomaly:self.volume_anomaly/sum,funding_extreme:self.funding_extreme/sum,order_book_imbalance:self.order_book_imbalance/sum,rsi_divergence:self.rsi_divergence/sum}}
pub fn adaptive(self,r:SignalReliability)->Self{if r.samples<MIN_SAMPLES_FOR_ADAPTATION{return self.normalized();}let maturity=((r.samples as f64-MIN_SAMPLES_FOR_ADAPTATION as f64)/170.0).clamp(0.0,1.0);fn factor(v:f64,m:f64)->f64{let raw=1.0+(v.clamp(0.0,100.0)-50.0)/100.0;1.0+(raw-1.0)*m}Self{volume_anomaly:self.volume_anomaly*factor(r.volume,maturity),funding_extreme:self.funding_extreme*factor(r.funding,maturity),order_book_imbalance:self.order_book_imbalance*factor(r.order_book,maturity),rsi_divergence:self.rsi_divergence*factor(r.rsi,maturity)}.bounded(WEIGHT_MIN,WEIGHT_MAX)}
fn bounded(self,min:f64,max:f64)->Self{let mut w=self.normalized();for _ in 0..4{w.volume_anomaly=w.volume_anomaly.clamp(min,max);w.funding_extreme=w.funding_extreme.clamp(min,max);w.order_book_imbalance=w.order_book_imbalance.clamp(min,max);w.rsi_divergence=w.rsi_divergence.clamp(min,max);w=w.normalized();}w}}
impl Default for ScoreWeights{fn default()->Self{Self{volume_anomaly:0.25,funding_extreme:0.25,order_book_imbalance:0.25,rsi_divergence:0.25}}}
fn env_weight(key:&str,default:f64)->f64{std::env::var(key).ok().and_then(|v|v.parse().ok()).unwrap_or(default)}
#[cfg(test)]mod tests{use super::*;#[test]fn normalizes_to_sum_one(){let w=ScoreWeights{volume_anomaly:1.0,funding_extreme:1.0,order_book_imbalance:1.0,rsi_divergence:1.0}.normalized();assert!((w.volume_anomaly+w.funding_extreme+w.order_book_imbalance+w.rsi_divergence-1.0).abs()<1e-9);}#[test]fn immature_history_does_not_adapt(){let b=ScoreWeights::default();assert_eq!(b.adaptive(SignalReliability{volume:100.0,funding:0.0,order_book:0.0,rsi:0.0,samples:10}),b);}#[test]fn mature_reliable_signal_gets_more_weight(){let w=ScoreWeights::default().adaptive(SignalReliability{volume:90.0,funding:30.0,order_book:30.0,rsi:30.0,samples:200});assert!(w.volume_anomaly>w.funding_extreme);assert!(w.volume_anomaly<=0.45);}}
