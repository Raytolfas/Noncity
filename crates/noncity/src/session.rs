use crate::config::{Config, ForwardingMode};
use crate::forwarding::{create_velocity_forwarding_data, VELOCITY_FORWARDING_CHANNEL};
use noncity_net::Connection;
use noncity_proto::packet::handshake::{HandshakeNextState, HandshakePacket};
use noncity_proto::packet::login::{
    LoginDisconnect, LoginPluginRequest, LoginPluginResponse, LoginStart, LoginSuccess,
};
use noncity_proto::packet::status::{
    PingPacket, ServerStatus, StatusDescription, StatusPlayers, StatusRequest, StatusResponse,
    StatusVersion,
};
use noncity_proto::uuid::offline_player_uuid;
use noncity_proto::State;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

pub struct Session {
    client: Connection,
    config: Arc<Config>,
    online_players: Arc<AtomicI32>,
    state: State,
}

impl Session {
    pub fn new(client: Connection, config: Arc<Config>, online_players: Arc<AtomicI32>) -> Self {
        Self {
            client,
            config,
            online_players,
            state: State::Handshake,
        }
    }

    pub async fn run(mut self) {
        if let Err(err) = self.handle_handshake().await {
            debug!(peer = %self.client.peer_addr(), "session ended: {err}");
        }
    }

    async fn handle_handshake(&mut self) -> anyhow::Result<()> {
        let raw = self.client.read_packet().await?;
        if raw.id != 0x00 {
            anyhow::bail!("expected handshake packet (0x00), got {:#x}", raw.id);
        }

        let handshake: HandshakePacket = raw.deserialize()?;
        debug!(
            peer = %self.client.peer_addr(),
            protocol = handshake.protocol_version.0,
            host = %handshake.server_address,
            next_state = ?handshake.next_state,
            "received handshake"
        );

        match handshake.next_state {
            HandshakeNextState::Status => {
                self.state = State::Status;
                self.handle_status(handshake.protocol_version.0).await
            }
            HandshakeNextState::Login => {
                self.state = State::Login;
                self.handle_login(handshake).await
            }
            HandshakeNextState::Transfer => {
                anyhow::bail!("transfer state not supported");
            }
        }
    }

    async fn handle_status(&mut self, protocol_version: i32) -> anyhow::Result<()> {
        let raw = self.client.read_packet().await?;
        if raw.id != 0x00 {
            anyhow::bail!("expected status request (0x00), got {:#x}", raw.id);
        }
        let _req: StatusRequest = raw.deserialize()?;

        let status = ServerStatus {
            version: StatusVersion {
                name: "Noncity".to_string(),
                protocol: protocol_version,
            },
            players: StatusPlayers {
                max: self.config.max_players,
                online: self.online_players.load(Ordering::Relaxed),
                sample: Vec::new(),
            },
            description: StatusDescription {
                text: self.config.motd.clone(),
            },
            favicon: None,
            enforces_secure_chat: Some(false),
        };

        let json = serde_json::to_string(&status)?;
        let response = StatusResponse { json };
        self.client.write(0x00, &response).await?;
        self.client.flush().await?;

        let ping_raw = self.client.read_packet().await?;
        if ping_raw.id != 0x01 {
            anyhow::bail!("expected ping packet (0x01), got {:#x}", ping_raw.id);
        }
        let ping: PingPacket = ping_raw.deserialize()?;
        self.client.write(0x01, &ping).await?;
        self.client.flush().await?;

        Ok(())
    }

    async fn handle_login(&mut self, handshake: HandshakePacket) -> anyhow::Result<()> {
        let raw_login_start = self.client.read_packet().await?;
        if raw_login_start.id != 0x00 {
            anyhow::bail!("expected login start (0x00), got {:#x}", raw_login_start.id);
        }

        let login_start: LoginStart = raw_login_start.deserialize()?;
        let player_name = login_start.username.clone();
        let player_uuid = login_start
            .player_uuid
            .unwrap_or_else(|| offline_player_uuid(&player_name));

        info!(
            player = %player_name,
            uuid = %player_uuid,
            peer = %self.client.peer_addr(),
            "player connecting"
        );

        let backend_addr = match self.config.servers.get(&self.config.default_server) {
            Some(&addr) => addr,
            None => {
                let disconnect = LoginDisconnect {
                    reason: r#"{"text":"Default backend server not configured"}"#.to_string(),
                };
                let _ = self.client.write(0x00, &disconnect).await;
                let _ = self.client.flush().await;
                anyhow::bail!("server '{}' not found", self.config.default_server);
            }
        };

        let mut backend = match self.connect_backend(backend_addr, &handshake, &login_start).await {
            Ok(b) => b,
            Err(err) => {
                error!(
                    player = %player_name,
                    server = %self.config.default_server,
                    "failed to connect to backend: {err}"
                );
                let disconnect = LoginDisconnect {
                    reason: format!(r#"{{"text":"Could not connect to backend server: {err}"}}"#),
                };
                let _ = self.client.write(0x00, &disconnect).await;
                let _ = self.client.flush().await;
                return Err(err);
            }
        };

        self.login_backend(&mut backend, player_uuid, &player_name)
            .await?;

        self.online_players.fetch_add(1, Ordering::Relaxed);
        info!(player = %player_name, "login successful, entering proxy pipe");

        let pipe_res = self.bridge_traffic(&mut backend).await;
        self.online_players.fetch_sub(1, Ordering::Relaxed);

        info!(player = %player_name, "player disconnected");
        pipe_res
    }

    async fn connect_backend(
        &self,
        backend_addr: SocketAddr,
        client_handshake: &HandshakePacket,
        login_start: &LoginStart,
    ) -> anyhow::Result<Connection> {
        let stream = TcpStream::connect(backend_addr).await?;
        stream.set_nodelay(true)?;
        let mut backend = Connection::new(stream, backend_addr);

        let mut backend_handshake = client_handshake.clone();
        if self.config.forwarding_mode == ForwardingMode::Legacy {
            backend_handshake.server_address = format!(
                "{}\0{}\0{}",
                backend_handshake.server_address,
                self.client.peer_addr().ip(),
                offline_player_uuid(&login_start.username).as_simple()
            );
        }

        backend.write(0x00, &backend_handshake).await?;
        backend.write(0x00, login_start).await?;
        backend.flush().await?;

        Ok(backend)
    }

    async fn login_backend(
        &mut self,
        backend: &mut Connection,
        player_uuid: uuid::Uuid,
        player_name: &str,
    ) -> anyhow::Result<()> {
        loop {
            let packet = backend.read_packet().await?;
            match packet.id {
                0x00 => {
                    let disconnect: LoginDisconnect = packet.deserialize()?;
                    self.client.write(0x00, &disconnect).await?;
                    self.client.flush().await?;
                    anyhow::bail!("backend disconnected login: {}", disconnect.reason);
                }
                0x01 => {
                    anyhow::bail!("backend requested encryption in proxy link");
                }
                0x02 => {
                    let success: LoginSuccess = packet.deserialize()?;
                    self.client.write(0x02, &success).await?;
                    self.client.flush().await?;
                    break;
                }
                0x03 => {
                    self.client.write_packet(packet).await?;
                    self.client.flush().await?;
                }
                0x04 => {
                    let req: LoginPluginRequest = packet.deserialize()?;
                    if req.channel == VELOCITY_FORWARDING_CHANNEL
                        && self.config.forwarding_mode == ForwardingMode::Velocity
                    {
                        let response_data = create_velocity_forwarding_data(
                            &self.config.forwarding_secret,
                            self.client.peer_addr(),
                            player_uuid,
                            player_name,
                        )?;
                        let response = LoginPluginResponse {
                            message_id: req.message_id,
                            successful: true,
                            data: Some(response_data),
                        };
                        backend.write(0x02, &response).await?;
                        backend.flush().await?;
                    } else {
                        let response = LoginPluginResponse {
                            message_id: req.message_id,
                            successful: false,
                            data: None,
                        };
                        backend.write(0x02, &response).await?;
                        backend.flush().await?;
                    }
                }
                _ => {
                    self.client.write_packet(packet).await?;
                    self.client.flush().await?;
                }
            }
        }
        Ok(())
    }

    async fn bridge_traffic(&mut self, backend: &mut Connection) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                client_packet = self.client.read_packet() => {
                    let packet = client_packet?;
                    backend.write_packet(packet).await?;
                    backend.flush().await?;
                }
                backend_packet = backend.read_packet() => {
                    let packet = backend_packet?;
                    self.client.write_packet(packet).await?;
                    self.client.flush().await?;
                }
            }
        }
    }
}
