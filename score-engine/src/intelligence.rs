use shared::Candle;
use crate::{market_regime, market_structure, MarketRegime, StructureState};

#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeResult { pub interval: String, pub score: f64, pub regime: MarketRegime, pub structure: StructureState }

pub fn multi_timeframe(inputs: &[(&str, &[Candle], f64)]) -> Vec<TimeframeResult> {
    inputs.iter().map(|(interval, candles, score)| TimeframeResult { interval: (*interval).to_string(), score: score.clamp(0.0, 100.0), regime: market_regime(candles).regime, structure: market_structure(candles).structure }).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationResult { pub correlation: f64, pub beta: f64, pub independence_score: f64 }

pub fn correlation(asset: &[f64], benchmark: &[f64]) -> CorrelationResult {
    let n = asset.len().min(benchmark.len());
    if n < 3 { return CorrelationResult { correlation: 0.0, beta: 0.0, independence_score: 0.0 }; }
    let a=&asset[asset.len()-n..]; let b=&benchmark[benchmark.len()-n..];
    let ma=a.iter().sum::<f64>()/n as f64; let mb=b.iter().sum::<f64>()/n as f64;
    let cov=a.iter().zip(b).map(|(x,y)|(x-ma)*(y-mb)).sum::<f64>()/n as f64;
    let va=a.iter().map(|x|(x-ma).powi(2)).sum::<f64>()/n as f64; let vb=b.iter().map(|x|(x-mb).powi(2)).sum::<f64>()/n as f64;
    let corr=if va>0.0&&vb>0.0 { cov/(va.sqrt()*vb.sqrt()) } else {0.0};
    CorrelationResult { correlation:corr.clamp(-1.0,1.0), beta:if vb>0.0 {cov/vb}else{0.0}, independence_score:((1.0-corr.abs())*100.0).clamp(0.0,100.0) }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnomalyResult { pub anomaly_score:f64, pub severity:AnomalySeverity }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum AnomalySeverity { Normal, Low, Medium, High, Extreme }
pub fn anomaly_zscore(history:&[f64], current:f64)->AnomalyResult { if history.len()<10 {return AnomalyResult{anomaly_score:0.0,severity:AnomalySeverity::Normal}} let m=history.iter().sum::<f64>()/history.len() as f64; let sd=(history.iter().map(|x|(x-m).powi(2)).sum::<f64>()/history.len() as f64).sqrt(); let z=if sd>0.0{((current-m)/sd).abs()}else{0.0}; let s=(z*25.0).clamp(0.0,100.0); let severity=if s>=90.0{AnomalySeverity::Extreme}else if s>=70.0{AnomalySeverity::High}else if s>=45.0{AnomalySeverity::Medium}else if s>=20.0{AnomalySeverity::Low}else{AnomalySeverity::Normal}; AnomalyResult{anomaly_score:s,severity} }

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Decision { StrongLong, Long, WatchLong, Neutral, WatchShort, Short, StrongShort, NoTrade }
#[derive(Debug, Clone, Copy, PartialEq)] pub struct MetaDecision { pub decision:Decision, pub signal_quality:f64, pub confidence:f64, pub risk:f64 }
pub fn meta_decision(score:f64, agreement:f64, data_quality:f64, risk:f64)->MetaDecision { if data_quality<50.0 || risk>=90.0 {return MetaDecision{decision:Decision::NoTrade,signal_quality:0.0,confidence:0.0,risk}} let quality=(agreement*0.6+data_quality*0.4).clamp(0.0,100.0); let confidence=(quality*(1.0-risk/100.0)).clamp(0.0,100.0); let d=if score>=85.0&&quality>=75.0{Decision::StrongLong}else if score>=70.0{Decision::Long}else if score>=58.0{Decision::WatchLong}else if score<=15.0&&quality>=75.0{Decision::StrongShort}else if score<=30.0{Decision::Short}else if score<=42.0{Decision::WatchShort}else{Decision::Neutral}; MetaDecision{decision:d,signal_quality:quality,confidence,risk} }

#[cfg(test)] mod tests { use super::*; #[test] fn perfect_correlation(){let r=correlation(&[1.,2.,3.,4.],&[2.,4.,6.,8.]);assert!((r.correlation-1.0).abs()<1e-9);} #[test] fn bad_data_blocks(){assert_eq!(meta_decision(95.,90.,20.,10.).decision,Decision::NoTrade);} #[test] fn anomaly_detects_outlier(){let h=vec![10.,11.,9.,10.,10.,11.,9.,10.,10.,10.];assert!(anomaly_zscore(&h,50.).anomaly_score>70.);} }
