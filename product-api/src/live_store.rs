use redis::AsyncCommands;use shared::{LiveIntelligence,Score};#[derive(Clone)]pub struct LiveStore{client:redis::Client}impl LiveStore{pub fn new(url:&str)->redis::RedisResult<Self>{Ok(Self{client:redis::Client::open(url)?})}pub async fn intelligence(&self,symbol:&str)->redis::RedisResult<Option<LiveIntelligence>>{let mut c=self.client.get_multiplexed_async_connection().await?;let raw:Option<String>=c.get(format!("intelligence:{}",symbol)).await?;Ok(raw.and_then(|x|serde_json::from_str(&x).ok()))}
    /// Reserved for a fallback path reading the raw score keys directly when no live intelligence snapshot is cached yet.
    #[allow(dead_code)]
    pub async fn score(&self,symbol:&str)->redis::RedisResult<Option<Score>>{let mut c=self.client.get_multiplexed_async_connection().await?;let keys=[format!("score:{}",symbol),format!("scores:{}",symbol),format!("latest_score:{}",symbol)];for key in keys{let raw:Option<String>=c.get(&key).await?;if let Some(raw)=raw{if let Ok(score)=serde_json::from_str::<Score>(&raw){return Ok(Some(score));}}}Ok(None)}pub async fn ping(&self)->bool{let Ok(mut c)=self.client.get_multiplexed_async_connection().await else{return false};redis::cmd("PING").query_async::<String>(&mut c).await.is_ok()}
    /// Discovers every symbol currently publishing live intelligence, by
    /// scanning for `intelligence:*` keys rather than relying on a
    /// hand-maintained list — so Market Scanner's "ALL" mode reflects
    /// whatever exchange-client is actually tracking right now.
    pub async fn live_symbols(&self)->redis::RedisResult<Vec<String>>{
        let mut c=self.client.get_multiplexed_async_connection().await?;
        let mut symbols=Vec::new();
        let mut iter:redis::AsyncIter<String>=c.scan_match("intelligence:*").await?;
        while let Some(key)=futures_util::StreamExt::next(&mut iter).await{
            if let Some(symbol)=key.strip_prefix("intelligence:"){symbols.push(symbol.to_string());}
        }
        drop(iter);
        symbols.sort();
        Ok(symbols)
    }
}
