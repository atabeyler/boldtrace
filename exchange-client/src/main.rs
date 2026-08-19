//! Standalone binary that runs the exchange-client market data pipeline.

use exchange_client::ExchangeClientConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber_init();

    let config = ExchangeClientConfig::from_env();
    tracing::info!(symbols = %config.symbols.join(","), "starting exchange-client");

    if let Err(err) = exchange_client::run(config).await {
        tracing::error!(error = %err, "exchange-client exited with an error");
        std::process::exit(1);
    }
}

fn tracing_subscriber_init() {
    let _ = tracing_subscriber::fmt::try_init();
}
