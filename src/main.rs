use std::net::SocketAddr;

use anyhow::Context;
use limited::{
    app::state::AppState, config::Config, infrastructure::redis::RedisInfrastructure,
    interfaces::http::routes,
};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("limited=info")))
        .with(fmt::layer())
        .init();

    let config = Config::from_env()?;
    let redis = RedisInfrastructure::connect(&config.redis_url).await?;
    let state = AppState::new(redis);
    let app = routes::router(state);

    let address: SocketAddr = config
        .bind_address
        .parse()
        .context("invalid BIND_ADDRESS")?;
    let listener = TcpListener::bind(address).await?;

    tracing::info!(%address, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}
