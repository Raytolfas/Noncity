use crate::auth::AuthManager;
use crate::config::Config;
use crate::limiter::RateLimiter;
use crate::plugin::PluginManager;
use crate::session::Session;
use noncity_net::Connection;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

pub struct Server {
    config: Arc<Config>,
    auth: Arc<AuthManager>,
    limiter: Arc<RateLimiter>,
    plugins: Arc<PluginManager>,
    online_players: Arc<AtomicI32>,
}

impl Server {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let auth = AuthManager::new()?;
        let limiter = RateLimiter::new(
            config.rate_limit_burst,
            config.rate_limit_refill,
            config.max_connections_per_ip,
        );
        let plugins = PluginManager::new("plugins");

        Ok(Self {
            config: Arc::new(config),
            auth: Arc::new(auth),
            limiter: Arc::new(limiter),
            plugins: Arc::new(plugins),
            online_players: Arc::new(AtomicI32::new(0)),
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        info!("listening on {}", self.config.bind);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let peer_ip = peer_addr.ip();
                    if !self.limiter.acquire(peer_ip) {
                        debug!(peer = %peer_addr, "connection dropped by rate limiter");
                        drop(stream);
                        continue;
                    }

                    let _ = stream.set_nodelay(true);
                    let conn = Connection::new(stream, peer_addr);
                    let session = Session::new(
                        conn,
                        Arc::clone(&self.config),
                        Arc::clone(&self.auth),
                        Arc::clone(&self.plugins),
                        Arc::clone(&self.online_players),
                    );
                    let limiter = Arc::clone(&self.limiter);

                    tokio::spawn(async move {
                        session.run().await;
                        limiter.release(peer_ip);
                    });
                }
                Err(err) => {
                    error!("accept error: {err}");
                }
            }
        }
    }
}
