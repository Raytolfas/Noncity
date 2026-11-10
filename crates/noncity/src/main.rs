mod auth;
mod command;
mod config;
mod forwarding;
mod server;
mod session;

use config::Config;
use server::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noncity=info,noncity_net=info,noncity_proto=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load_or_create("config.toml").await?;
    let server = Server::new(config)?;
    server.run().await
}
