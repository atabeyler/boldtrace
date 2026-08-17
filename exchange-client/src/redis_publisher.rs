//! Publishes deserialized market data to Redis pub/sub channels.

use redis::AsyncCommands;
use shared::{Candle, FundingRate, OrderBookSnapshot};

use crate::error::Result;

/// Thin wrapper around an async Redis multiplexed connection that publishes
/// `shared` domain types as JSON on well-known channels.
pub struct RedisPublisher {
    connection: redis::aio::MultiplexedConnection,
}

impl RedisPublisher {
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self { connection })
    }

    pub async fn publish_candle(&mut self, candle: &Candle) -> Result<()> {
        let channel = format!("candles:{}:{}", candle.symbol, candle.interval);
        let payload = serde_json::to_string(candle)?;
        let _: () = self.connection.publish(channel, payload).await?;
        Ok(())
    }

    pub async fn publish_order_book(&mut self, snapshot: &OrderBookSnapshot) -> Result<()> {
        let channel = format!("orderbook:{}", snapshot.symbol);
        let payload = serde_json::to_string(snapshot)?;
        let _: () = self.connection.publish(channel, payload).await?;
        Ok(())
    }

    pub async fn publish_funding_rate(&mut self, funding_rate: &FundingRate) -> Result<()> {
        let channel = format!("funding:{}", funding_rate.symbol);
        let payload = serde_json::to_string(funding_rate)?;
        let _: () = self.connection.publish(channel, payload).await?;
        Ok(())
    }
}
