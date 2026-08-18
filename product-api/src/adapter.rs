use score_engine::intelligence::{meta_decision,Decision,MetaDecision};use shared::Score;
/// Reserved for a future on-demand endpoint that computes a decision directly
/// from a raw `Score`, bypassing the live Redis snapshot pipeline.
#[allow(dead_code)]
#[derive(Debug,Clone)]pub struct ProductDecision{pub decision:String,pub confidence:f64,pub risk:f64,pub quality:f64,pub reasons:Vec<String>}
#[allow(dead_code)]
pub fn from_score(score:&Score,data_quality:f64,risk:f64)->ProductDecision{let components=[score.volume_anomaly,score.funding_extreme,score.order_book_imbalance,score.rsi_divergence];let directional=components.iter().filter(|v|**v>=50.0).count() as f64;let agreement=directional/components.len() as f64*100.0;let meta=meta_decision(score.value,agreement,data_quality,risk);ProductDecision{decision:label(meta.decision).into(),confidence:meta.confidence,risk:meta.risk,quality:meta.signal_quality,reasons:reasons(score,&meta)}}
fn label(d:Decision)->&'static str{match d{Decision::StrongLong|Decision::Long=>"LONG",Decision::WatchLong|Decision::Neutral|Decision::WatchShort=>"WATCH",Decision::Short|Decision::StrongShort=>"SHORT",Decision::NoTrade=>"NO TRADE"}}
fn reasons(s:&Score,m:&MetaDecision)->Vec<String>{
    let mut r=Vec::new();
    if s.order_book_imbalance>=60.0{r.push("Order-book imbalance supports the directional score.".into())}
    if s.volume_anomaly>=60.0{r.push("Volume anomaly confirms elevated market participation.".into())}
    if s.funding_extreme>=70.0{r.push("Funding conditions are extreme and increase derivatives risk.".into())}
    if s.rsi_divergence>=60.0{r.push("RSI divergence contributes directional evidence.".into())}
    if m.risk>=70.0{r.push("Risk is elevated; position decisions require defensive handling.".into())}
    if r.is_empty(){r.push("No component has enough standalone strength; consensus remains limited.".into())}
    r
}
#[cfg(test)]mod tests{use super::*;#[test]fn bad_quality_blocks_trade(){let s=Score{symbol:"BTCUSDT".into(),timestamp:1,value:90.,volume_anomaly:80.,funding_extreme:40.,order_book_imbalance:80.,rsi_divergence:70.};let d=from_score(&s,20.,10.);assert_eq!(d.decision,"NO TRADE");assert_eq!(d.confidence,0.0)}#[test]fn strong_score_maps_long(){let s=Score{symbol:"BTCUSDT".into(),timestamp:1,value:90.,volume_anomaly:80.,funding_extreme:60.,order_book_imbalance:85.,rsi_divergence:70.};let d=from_score(&s,90.,15.);assert_eq!(d.decision,"LONG");assert!(d.confidence>60.)}}
