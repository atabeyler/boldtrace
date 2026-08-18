export type Decision='LONG'|'SHORT'|'WATCH'|'NO TRADE';export type Health='healthy'|'degraded'|'offline';
export interface EngineEvidence{name:string;score:number;state:string;weight:number;reliability:'Low'|'Medium'|'High'}
export interface MarketDecision{symbol:string;price:number;decision:Decision;confidence:number;risk:number;change24h:number;regime:string;quality:number;engines:EngineEvidence[];reasons:string[];timestamp:string}
export interface PerformanceSummary{horizon:string;winRate:number;samples:number;reliability:string}
export interface ServiceHealth{name:string;status:Health;freshnessMs?:number;latencyMs?:number}
export interface IntelligenceSnapshot{market:MarketDecision;performance:PerformanceSummary[];services:ServiceHealth[]}
export interface DecisionHistoryItem{id:string;symbol:string;decision:Decision;confidence:number;horizon:string;realizedReturn?:number;outcome?:'WIN'|'LOSS'|'PENDING';createdAt:string}
