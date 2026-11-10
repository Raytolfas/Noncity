use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardingMode {
    None,
    Legacy,
    Velocity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind: SocketAddr,
    pub motd: String,
    pub max_players: i32,
    pub online_mode: bool,
    pub compression_threshold: i32,
    pub default_server: String,
    pub forwarding_mode: ForwardingMode,
    pub forwarding_secret: String,
    pub tablist_header: String,
    pub tablist_footer: String,
    pub rate_limit_burst: f64,
    pub rate_limit_refill: f64,
    pub max_connections_per_ip: usize,
    pub servers: HashMap<String, SocketAddr>,
}

impl Default for Config {
    fn default() -> Self {
        let mut servers = HashMap::new();
        servers.insert("lobby".to_string(), "127.0.0.1:25566".parse().unwrap());

        Self {
            bind: "0.0.0.0:25565".parse().unwrap(),
            motd: "§bNoncity §7- High performance Minecraft proxy".to_string(),
            max_players: 1000,
            online_mode: false,
            compression_threshold: 256,
            default_server: "lobby".to_string(),
            forwarding_mode: ForwardingMode::Velocity,
            forwarding_secret: "change_me_to_a_secure_random_secret".to_string(),
            tablist_header: "§b§lNONCITY PROXY\n§7play.example.com".to_string(),
            tablist_footer: "\n§7High Performance Minecraft Proxy (Tokio Core)".to_string(),
            rate_limit_burst: 20.0,
            rate_limit_refill: 5.0,
            max_connections_per_ip: 5,
            servers,
        }
    }
}

impl Config {
    pub async fn load_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let p = path.as_ref();
        if p.exists() {
            let content = tokio::fs::read_to_string(p).await?;
            let cfg: Config = toml::from_str(&content)?;
            Ok(cfg)
        } else {
            let cfg = Config::default();
            let content = toml::to_string_pretty(&cfg)?;
            tokio::fs::write(p, content).await?;
            Ok(cfg)
        }
    }
}
