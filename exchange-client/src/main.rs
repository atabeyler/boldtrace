//! Standalone binary that runs the exchange-client market data pipeline.

use exchange_client::ExchangeClientConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber_init();

    let config = ExchangeClientConfig::from_env();
    let config = match config.resolve_symbols().await {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(error = %err, "failed to discover symbol universe");
            std::process::exit(1);
        }
    };
    tracing::info!(
        provider = config.provider.as_str(),
        count = config.symbols.len(),
        symbols = %config.symbols.join(","),
        "starting exchange-client"
    );

    if let Err(err) = exchange_client::run(config).await {
        tracing::error!(error = %err, "exchange-client exited with an error");
        std::process::exit(1);
    }
}

fn tracing_subscriber_init() {
    let _ = tracing_subscriber::fmt::try_init();
}
