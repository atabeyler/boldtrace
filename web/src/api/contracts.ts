export type Decision='LONG'|'SHORT'|'WATCH'|'NO TRADE';export type Health='healthy'|'degraded'|'offline';
export interface EngineEvidence{name:string;score:number;state:string;weight:number;reliability:'Low'|'Medium'|'High'}
export interface MarketDecision{symbol:string;price:number;decision:Decision;confidence:number;risk:number;change24h:number;regime:string;quality:number;engines:EngineEvidence[];reasons:string[];timestamp:string}
export interface PerformanceSummary{horizon:string;winRate:number;samples:number;reliability:string}
export interface ServiceHealth{name:string;status:Health;freshnessMs?:number;latencyMs?:number}
export interface IntelligenceSnapshot{market:MarketDecision;performance:PerformanceSummary[];services:ServiceHealth[]}
export interface DecisionHistoryItem{id:string;symbol:string;decision:Decision;confidence:number;horizon:string;realizedReturn?:number;outcome?:'WIN'|'LOSS'|'PENDING';createdAt:string}
export interface Account{id:string;userCode:string;email:string;firstName:string;lastName:string;language:string;status:'pending'|'approved'|'rejected';isAdmin:boolean}
export interface PendingUser{id:string;userCode:string;email:string;firstName:string;lastName:string;country:string;nationalId:string;createdAt:string}
export interface LocationAlert{id:string;email:string;firstName:string;lastName:string;expectedCountry:string;detectedCountry:string;ip:string;createdAt:string}
export interface RegisterInput{firstName:string;lastName:string;userCode:string;country:string;nationalId:string;email:string;password:string;language:string;termsAccepted:boolean}
export interface LoginInput{email:string;password:string;rememberMe:boolean;browserLat?:number;browserLon?:number}
export interface ApiErrorBody{error:string}
