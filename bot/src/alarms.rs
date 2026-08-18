//! Edge-triggered score alarms plus smart decision alerts.
use std::{collections::HashMap, sync::Mutex};
use score_engine::{Decision, MetaDecision};
struct Alarm { telegram_id:i64, symbol:String, threshold:f64, was_above:bool }
#[derive(Default)] pub struct AlarmRegistry { alarms:Mutex<Vec<Alarm>> }
impl AlarmRegistry {
 pub fn set(&self,telegram_id:i64,symbol:&str,threshold:f64){let mut a=self.alarms.lock().expect("alarm registry lock poisoned");a.retain(|x|!(x.telegram_id==telegram_id&&x.symbol==symbol));a.push(Alarm{telegram_id,symbol:symbol.to_string(),threshold,was_above:false});}
 pub fn crossed(&self,symbol:&str,score:f64)->Vec<(i64,f64)>{let mut a=self.alarms.lock().expect("alarm registry lock poisoned");let mut t=vec![];for x in a.iter_mut().filter(|x|x.symbol==symbol){let above=score>=x.threshold;if above&&!x.was_above{t.push((x.telegram_id,x.threshold));}x.was_above=above;}t}
}
#[derive(Debug,Clone,PartialEq)] pub struct SmartAlert { pub symbol:String,pub decision:Decision,pub confidence:f64,pub risk:f64 }
#[derive(Default)] pub struct SmartAlertGate { last:Mutex<HashMap<String,(Decision,i64)>> }
impl SmartAlertGate {
 pub fn evaluate(&self,symbol:&str,d:&MetaDecision,now_ms:i64,cooldown_ms:i64)->Option<SmartAlert>{
  if matches!(d.decision,Decision::Neutral|Decision::NoTrade)||d.confidence<60.0||d.risk>=80.0{return None;}
  let mut last=self.last.lock().expect("smart alert lock poisoned");
  if let Some((previous,at))=last.get(symbol){if *previous==d.decision&&now_ms-*at<cooldown_ms{return None;}}
  last.insert(symbol.to_string(),(d.decision,now_ms));
  Some(SmartAlert{symbol:symbol.to_string(),decision:d.decision,confidence:d.confidence,risk:d.risk})
 }
}
#[cfg(test)] mod tests {use super::*;#[test]fn triggers_once_on_upward_crossing(){let r=AlarmRegistry::default();r.set(1,"BTCUSDT",70.0);assert_eq!(r.crossed("BTCUSDT",75.0),vec![(1,70.0)]);assert!(r.crossed("BTCUSDT",80.0).is_empty());}#[test]fn smart_alert_respects_cooldown(){let g=SmartAlertGate::default();let d=MetaDecision{decision:Decision::Long,signal_quality:80.0,confidence:75.0,risk:20.0};assert!(g.evaluate("BTCUSDT",&d,1000,60000).is_some());assert!(g.evaluate("BTCUSDT",&d,2000,60000).is_none());}#[test]fn risky_signal_is_blocked(){let g=SmartAlertGate::default();let d=MetaDecision{decision:Decision::StrongLong,signal_quality:90.0,confidence:90.0,risk:90.0};assert!(g.evaluate("BTCUSDT",&d,1000,60000).is_none());}}
