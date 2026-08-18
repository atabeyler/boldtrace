use crate::{Decision, MetaDecision};

#[derive(Debug, Clone, PartialEq)]
pub struct Explanation {
    pub headline: String,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn explain(decision: &MetaDecision, factors: &[(&str, f64)]) -> Explanation {
    let headline = match decision.decision {
        Decision::StrongLong => "Strong long setup",
        Decision::Long => "Long setup",
        Decision::WatchLong => "Watch long",
        Decision::Neutral => "Neutral",
        Decision::WatchShort => "Watch short",
        Decision::Short => "Short setup",
        Decision::StrongShort => "Strong short setup",
        Decision::NoTrade => "No trade",
    }.to_string();
    let mut ranked = factors.to_vec();
    ranked.sort_by(|a,b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    let reasons = ranked.iter().take(3).map(|(name,value)| format!("{name}: {value:.1}")).collect();
    let mut warnings = Vec::new();
    if decision.risk >= 60.0 { warnings.push(format!("elevated risk: {:.1}", decision.risk)); }
    if decision.signal_quality < 60.0 { warnings.push(format!("weak agreement: {:.1}", decision.signal_quality)); }
    if decision.confidence < 50.0 { warnings.push(format!("low confidence: {:.1}", decision.confidence)); }
    Explanation { headline, reasons, warnings }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn explains_no_trade_risk() {
        let d=MetaDecision{decision:Decision::NoTrade,signal_quality:40.0,confidence:20.0,risk:95.0};
        let e=explain(&d,&[("order_flow",80.0),("regime",50.0)]);
        assert_eq!(e.headline,"No trade"); assert!(!e.warnings.is_empty());
    }
}
