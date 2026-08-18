//! Quality, calibration, adaptive weighting and risk primitives.

#[derive(Debug, Clone, PartialEq)]
pub struct DataSourceHealth { pub name:String, pub age_ms:i64, pub max_age_ms:i64, pub present:bool }
#[derive(Debug, Clone, PartialEq)]
pub struct DataQualityResult { pub score:f64, pub stale_sources:Vec<String>, pub missing_sources:Vec<String> }
pub fn data_quality(sources:&[DataSourceHealth])->DataQualityResult { if sources.is_empty(){return DataQualityResult{score:0.0,stale_sources:vec![],missing_sources:vec!["all".into()]};} let mut stale=vec![];let mut missing=vec![];let mut good=0.0; for s in sources {if !s.present{missing.push(s.name.clone())}else if s.age_ms<0||s.age_ms>s.max_age_ms{stale.push(s.name.clone())}else{good+=1.0}} DataQualityResult{score:(good/sources.len() as f64*100.0).clamp(0.0,100.0),stale_sources:stale,missing_sources:missing} }

#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum ConfidenceLevel{InsufficientData,Low,Medium,High,VeryHigh}
#[derive(Debug,Clone,Copy,PartialEq)] pub struct CalibrationResult{pub probability:f64,pub confidence:ConfidenceLevel,pub sample_size:usize}
pub fn empirical_calibration(outcomes:&[bool])->CalibrationResult {let n=outcomes.len();if n<30{return CalibrationResult{probability:0.0,confidence:ConfidenceLevel::InsufficientData,sample_size:n}} let wins=outcomes.iter().filter(|x|**x).count();let p=wins as f64/n as f64;let c=if n>=1000{ConfidenceLevel::VeryHigh}else if n>=300{ConfidenceLevel::High}else if n>=100{ConfidenceLevel::Medium}else{ConfidenceLevel::Low};CalibrationResult{probability:p,confidence:c,sample_size:n}}

#[derive(Debug,Clone,Copy,PartialEq)] pub struct AdaptiveWeights{pub regime:f64,pub order_flow:f64,pub derivatives:f64,pub volatility:f64,pub structure:f64}
impl AdaptiveWeights {pub fn bounded(mut self,min:f64,max:f64)->Self{self.regime=self.regime.clamp(min,max);self.order_flow=self.order_flow.clamp(min,max);self.derivatives=self.derivatives.clamp(min,max);self.volatility=self.volatility.clamp(min,max);self.structure=self.structure.clamp(min,max);let s=self.regime+self.order_flow+self.derivatives+self.volatility+self.structure;if s>0.0{self.regime/=s;self.order_flow/=s;self.derivatives/=s;self.volatility/=s;self.structure/=s;}self}}

#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum RiskLevel{Low,Medium,High,Extreme}
#[derive(Debug,Clone,Copy,PartialEq)] pub struct RiskResult{pub score:f64,pub level:RiskLevel}
pub fn risk(volatility:f64,liquidity:f64,data_quality:f64,timeframe_conflict:f64,anomaly:f64)->RiskResult{let score=(volatility*0.25+liquidity*0.25+(100.0-data_quality)*0.30+timeframe_conflict*0.10+anomaly*0.10).clamp(0.0,100.0);let level=if score>=80.0{RiskLevel::Extreme}else if score>=60.0{RiskLevel::High}else if score>=30.0{RiskLevel::Medium}else{RiskLevel::Low};RiskResult{score,level}}

#[derive(Debug,Clone,Copy,PartialEq)] pub struct SignalQuality{pub score:f64,pub agreement:f64,pub conflict:f64,pub confirmations:u8}
pub fn signal_quality(evidence:&[f64])->SignalQuality{if evidence.is_empty(){return SignalQuality{score:0.0,agreement:0.0,conflict:100.0,confirmations:0}}let pos=evidence.iter().filter(|x|**x>0.0).count();let neg=evidence.iter().filter(|x|**x<0.0).count();let majority=pos.max(neg);let agreement=majority as f64/evidence.len() as f64*100.0;let conflict=100.0-agreement;let magnitude=evidence.iter().map(|x|x.abs().min(100.0)).sum::<f64>()/evidence.len() as f64;SignalQuality{score:(agreement*0.7+magnitude*0.3).clamp(0.0,100.0),agreement,conflict,confirmations:majority as u8}}

#[cfg(test)] mod tests{use super::*;#[test]fn stale_data_reduces_quality(){let r=data_quality(&[DataSourceHealth{name:"candle".into(),age_ms:10,max_age_ms:100,present:true},DataSourceHealth{name:"oi".into(),age_ms:500,max_age_ms:100,present:true}]);assert_eq!(r.score,50.0);assert_eq!(r.stale_sources,vec!["oi"]);}#[test]fn calibration_requires_samples(){assert_eq!(empirical_calibration(&[true;10]).confidence,ConfidenceLevel::InsufficientData);}#[test]fn extreme_inputs_raise_risk(){assert_eq!(risk(100.,100.,0.,100.,100.).level,RiskLevel::Extreme);}#[test]fn aligned_evidence_is_quality(){assert!(signal_quality(&[80.,70.,60.]).score>80.0);}}
