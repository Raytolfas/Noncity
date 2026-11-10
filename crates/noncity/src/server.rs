use crate::auth::AuthManager;
use crate::config::Config;
use crate::session::Session;
use noncity_net::Connection;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct Server {
    config: Arc<Config>,
    auth: Arc<AuthManager>,
    online_players: Arc<AtomicI32>,
}

impl Server {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let auth = AuthManager::new()?;
        Ok(Self {
            config: Arc::new(config),
            auth: Arc::new(auth),
            online_players: Arc::new(AtomicI32::new(0)),
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        info!("listening on {}", self.config.bind);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let _ = stream.set_nodelay(true);
                    let conn = Connection::new(stream, peer_addr);
                    let session = Session::new(
                        conn,
                        Arc::clone(&self.config),
                        Arc::clone(&self.auth),
                        Arc::clone(&self.online_players),
                    );

                    tokio::spawn(async move {
                        session.run().await;
                    });
                }
                Err(err) => {
                    error!("accept error: {err}");
                }
            }
        }
    }
}
