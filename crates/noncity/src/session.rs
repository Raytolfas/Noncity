use crate::auth::{compute_server_hash, AuthManager, MojangProfileProperty};
use crate::command::{extract_proxy_command, handle_tab_complete, make_chat_json, ProxyCommand};
use crate::config::{Config, ForwardingMode};
use crate::forwarding::{create_velocity_forwarding_data, VELOCITY_FORWARDING_CHANNEL};
use crate::plugin::PluginManager;
use noncity_net::{Connection, RawPacket};
use noncity_proto::packet::handshake::{HandshakeNextState, HandshakePacket};
use bytes::Bytes;
use noncity_proto::packet::login::{
    EncryptionRequest, EncryptionResponse, LoginDisconnect, LoginPluginRequest,
    LoginPluginResponse, LoginStart, LoginSuccess, SetCompression,
};
use noncity_proto::packet::play::{
    command_suggestions_packet_id, system_chat_packet_id, translate_clientbound_id,
    translate_serverbound_id, SystemChatPacket,
};
use noncity_proto::packet::status::{
    PingPacket, ServerStatus, StatusDescription, StatusPlayers, StatusRequest, StatusResponse,
    StatusVersion,
};
use noncity_proto::uuid::offline_player_uuid;
use noncity_proto::varint::VarInt;
use noncity_proto::State;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, info};
use uuid::Uuid;

pub struct Session {
    client: Connection,
    config: Arc<Config>,
    auth: Arc<AuthManager>,
    plugins: Arc<PluginManager>,
    online_players: Arc<AtomicI32>,
    state: State,
    protocol_version: i32,
    current_server: String,
    player_name: String,
    player_uuid: Uuid,
    player_properties: Vec<MojangProfileProperty>,
}

impl Session {
    pub fn new(
        client: Connection,
        config: Arc<Config>,
        auth: Arc<AuthManager>,
        plugins: Arc<PluginManager>,
        online_players: Arc<AtomicI32>,
    ) -> Self {
        Self {
            client,
            config,
            auth,
            plugins,
            online_players,
            state: State::Handshake,
            protocol_version: 0,
            current_server: String::new(),
            player_name: String::new(),
            player_uuid: Uuid::nil(),
            player_properties: Vec::new(),
        }
    }

    pub async fn run(mut self) {
        if let Err(err) = self.handle_handshake().await {
            debug!(peer = %self.client.peer_addr(), "session finished: {err}");
        }
    }

    async fn handle_handshake(&mut self) -> anyhow::Result<()> {
        let raw = self.client.read_packet().await?;
        if raw.id != 0x00 {
            anyhow::bail!("expected handshake packet (0x00), got {:#x}", raw.id);
        }

        let handshake: HandshakePacket = raw.deserialize()?;
        self.protocol_version = handshake.protocol_version.0;

        debug!(
            peer = %self.client.peer_addr(),
            protocol = self.protocol_version,
            host = %handshake.server_address,
            next_state = ?handshake.next_state,
            "received handshake"
        );

        match handshake.next_state {
            HandshakeNextState::Status => {
                self.state = State::Status;
                self.handle_status().await
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

    async fn handle_status(&mut self) -> anyhow::Result<()> {
        let raw = self.client.read_packet().await?;
        if raw.id != 0x00 {
            anyhow::bail!("expected status request (0x00), got {:#x}", raw.id);
        }
        let _req: StatusRequest = raw.deserialize()?;

        let version_name = if self.config.cross_version {
            "Noncity 1.7.x - 1.26.x".to_string()
        } else {
            "Noncity".to_string()
        };

        let status = ServerStatus {
            version: StatusVersion {
                name: version_name,
                protocol: self.protocol_version,
            },
            players: StatusPlayers {
                max: self.config.max_players,
                online: self.online_players.load(Ordering::Relaxed),
                sample: Vec::new(),
            },
            description: StatusDescription {
                text: self.config.motd.clone(),
            },
            favicon: Some("data:image/png;base64,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".to_string()),
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

    async fn handle_login(&mut self, client_handshake: HandshakePacket) -> anyhow::Result<()> {
        if self.config.cross_version {
            if self.protocol_version < 4 {
                let disconnect = LoginDisconnect {
                    reason: r#"{"text":"§cOutdated client! Noncity supports Minecraft 1.7.2 through 1.26.x"}"#.to_string(),
                };
                let _ = self.client.write(0x00, &disconnect).await;
                let _ = self.client.flush().await;
                anyhow::bail!("client protocol {} is below minimum supported 1.7.2", self.protocol_version);
            }
        } else if self.protocol_version < 764 {
            let disconnect = LoginDisconnect {
                reason: r#"{"text":"§cCross-version mode is disabled. Please connect using Minecraft 1.20.2 - 1.26.x"}"#.to_string(),
            };
            let _ = self.client.write(0x00, &disconnect).await;
            let _ = self.client.flush().await;
            anyhow::bail!("client protocol {} rejected (cross-version disabled)", self.protocol_version);
        }

        let raw_login_start = self.client.read_packet().await?;
        if raw_login_start.id != 0x00 {
            anyhow::bail!("expected login start (0x00), got {:#x}", raw_login_start.id);
        }

        let login_start: LoginStart = raw_login_start.deserialize()?;
        self.player_name = login_start.username.clone();

        let client_ip = self.client.peer_addr().ip().to_string();
        if !self.plugins.on_player_login(&self.player_name, &client_ip) {
            let disconnect = LoginDisconnect {
                reason: r#"{"text":"§cConnection denied by Noncity plugin"}"#.to_string(),
            };
            let _ = self.client.write(0x00, &disconnect).await;
            let _ = self.client.flush().await;
            anyhow::bail!("login rejected by plugin");
        }

        if self.config.online_mode {
            let verify_token = self.auth.generate_verify_token();
            let should_authenticate = if self.protocol_version >= 766 {
                Some(true)
            } else {
                None
            };
            let enc_request = EncryptionRequest {
                server_id: "".to_string(),
                public_key: self.auth.public_key_der().to_vec(),
                verify_token: verify_token.to_vec(),
                should_authenticate,
            };
            self.client.write(0x01, &enc_request).await?;
            self.client.flush().await?;

            let enc_resp_raw = self.client.read_packet().await?;
            if enc_resp_raw.id != 0x01 {
                anyhow::bail!("expected encryption response (0x01), got {:#x}", enc_resp_raw.id);
            }
            let enc_resp: EncryptionResponse = enc_resp_raw.deserialize()?;

            let decrypted_token = self.auth.decrypt(&enc_resp.verify_token)?;
            if decrypted_token != verify_token {
                anyhow::bail!("verify token mismatch");
            }

            let shared_secret = self.auth.decrypt(&enc_resp.shared_secret)?;
            if shared_secret.len() != 16 {
                anyhow::bail!("invalid shared secret length");
            }
            let mut key = [0u8; 16];
            key.copy_from_slice(&shared_secret);

            self.client.enable_encryption(&key);

            let server_hash = compute_server_hash("", &shared_secret, self.auth.public_key_der());
            let profile = self
                .auth
                .verify_mojang(&self.player_name, &server_hash, None)
                .await?;

            self.player_uuid = profile.id;
            self.player_name = profile.name;
            self.player_properties = profile.properties;
        } else {
            self.player_uuid = login_start
                .player_uuid
                .unwrap_or_else(|| offline_player_uuid(&self.player_name));
        }

        info!(
            player = %self.player_name,
            uuid = %self.player_uuid,
            peer = %self.client.peer_addr(),
            "player authenticated"
        );

        if self.config.compression_threshold >= 0 {
            let set_comp = SetCompression {
                threshold: VarInt(self.config.compression_threshold),
            };
            self.client.write(0x03, &set_comp).await?;
            self.client.flush().await?;
            self.client.set_compression(self.config.compression_threshold);
        }

        let default_target = self.config.default_server.clone();
        let mut backend = match self.connect_backend(&default_target, &client_handshake).await {
            Ok(b) => b,
            Err(err) => {
                let disconnect = LoginDisconnect {
                    reason: format!(r#"{{"text":"§cCould not connect to '{}': {}"}}"#, default_target, err),
                };
                let _ = self.client.write(0x00, &disconnect).await;
                let _ = self.client.flush().await;
                return Err(err);
            }
        };

        let login_success_pkt = self.login_success_backend(&mut backend).await?;
        self.current_server = default_target;

        if self.protocol_version < 776 {
            let client_login_success = LoginSuccess {
                uuid: self.player_uuid,
                username: self.player_name.clone(),
            };
            self.client.write(0x02, &client_login_success).await?;
        } else {
            self.client.write_packet(login_success_pkt).await?;
        }
        self.client.flush().await?;

        if self.protocol_version >= 764 {
            let ack = self.client.read_packet().await?;
            backend.write_packet(ack).await?;
            backend.flush().await?;
        } else {
            let ack = RawPacket {
                id: 0x03,
                payload: Bytes::new(),
            };
            backend.write_packet(ack).await?;
            backend.flush().await?;

            loop {
                let pkt = backend.read_packet().await?;
                if pkt.id == 0x03 {
                    let client_ack = RawPacket {
                        id: 0x03,
                        payload: Bytes::new(),
                    };
                    backend.write_packet(client_ack).await?;
                    backend.flush().await?;
                    break;
                }
            }
        }

        self.online_players.fetch_add(1, Ordering::Relaxed);
        info!(player = %self.player_name, server = %self.current_server, "player joined proxy");

        let pipe_res = self.play_loop(&mut backend, &client_handshake).await;
        self.online_players.fetch_sub(1, Ordering::Relaxed);

        info!(player = %self.player_name, "player disconnected");
        pipe_res
    }

    async fn connect_backend(
        &self,
        server_name: &str,
        client_handshake: &HandshakePacket,
    ) -> anyhow::Result<Connection> {
        let backend_addr = match self.config.servers.get(server_name) {
            Some(&addr) => addr,
            None => anyhow::bail!("server '{}' is not registered in configuration", server_name),
        };

        let stream = TcpStream::connect(backend_addr).await?;
        stream.set_nodelay(true)?;
        let mut backend = Connection::new(stream, backend_addr);

        let mut backend_handshake = client_handshake.clone();
        if self.config.cross_version {
            backend_handshake.protocol_version = VarInt(self.config.backend_protocol);
        }
        if self.config.forwarding_mode == ForwardingMode::Legacy {
            backend_handshake.server_address = format!(
                "{}\0{}\0{}",
                backend_handshake.server_address,
                self.client.peer_addr().ip(),
                self.player_uuid.as_simple()
            );
        }

        backend.write(0x00, &backend_handshake).await?;
        let backend_login_start = LoginStart {
            username: self.player_name.clone(),
            player_uuid: Some(self.player_uuid),
        };
        backend.write(0x00, &backend_login_start).await?;
        backend.flush().await?;

        Ok(backend)
    }

    async fn login_success_backend(&mut self, backend: &mut Connection) -> anyhow::Result<RawPacket> {
        loop {
            let packet = backend.read_packet().await?;
            match packet.id {
                0x00 => {
                    let disconnect: LoginDisconnect = packet.deserialize()?;
                    anyhow::bail!("backend rejected login: {}", disconnect.reason);
                }
                0x01 => {
                    anyhow::bail!("backend requested encryption on internal proxy link");
                }
                0x02 => {
                    return Ok(packet);
                }
                0x03 => {
                    let comp: SetCompression = packet.deserialize()?;
                    backend.set_compression(comp.threshold.0);
                }
                0x04 => {
                    let req: LoginPluginRequest = packet.deserialize()?;
                    if req.channel == VELOCITY_FORWARDING_CHANNEL
                        && self.config.forwarding_mode == ForwardingMode::Velocity
                    {
                        let response_data = create_velocity_forwarding_data(
                            &self.config.forwarding_secret,
                            self.client.peer_addr(),
                            self.player_uuid,
                            &self.player_name,
                            &self.player_properties,
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
                _ => {}
            }
        }
    }

    async fn send_chat_message(&mut self, message: &str) -> anyhow::Result<()> {
        let packet_id = system_chat_packet_id(self.protocol_version);
        let content = if self.protocol_version >= 765 {
            message.to_string()
        } else {
            make_chat_json(message)
        };
        let chat = SystemChatPacket {
            content,
            overlay: false,
            protocol_version: self.protocol_version,
        };
        self.client.write(packet_id, &chat).await?;
        self.client.flush().await?;
        Ok(())
    }

    async fn play_loop(
        &mut self,
        backend: &mut Connection,
        client_handshake: &HandshakePacket,
    ) -> anyhow::Result<()> {
        let server_names: Vec<String> = self.config.servers.keys().cloned().collect();

        loop {
            tokio::select! {
                client_res = self.client.read_packet() => {
                    let packet = client_res?;

                    if let Some(tab_resp) = handle_tab_complete(&packet, &server_names, self.protocol_version) {
                        let packet_id = command_suggestions_packet_id(self.protocol_version);
                        let _ = self.client.write(packet_id, &tab_resp).await;
                        let _ = self.client.flush().await;
                        continue;
                    }

                    if let Some(cmd) = extract_proxy_command(&packet, self.protocol_version) {
                        match cmd {
                            ProxyCommand::Server(None) => {
                                let mut list: Vec<&str> = self.config.servers.keys().map(|s| s.as_str()).collect();
                                list.sort();
                                let msg = format!(
                                    "§b[Noncity] §7Current: §a{}§7. Available: §e{}",
                                    self.current_server,
                                    list.join(", ")
                                );
                                let _ = self.send_chat_message(&msg).await;
                                continue;
                            }
                            ProxyCommand::Server(Some(target)) => {
                                if target == self.current_server {
                                    let _ = self.send_chat_message(&format!("§cAlready connected to '{}'", target)).await;
                                    continue;
                                }
                                if !self.config.servers.contains_key(&target) {
                                    let _ = self.send_chat_message(&format!("§cServer '{}' does not exist", target)).await;
                                    continue;
                                }
                                let _ = self.send_chat_message(&format!("§7Connecting to §b{}§7...", target)).await;
                                match self.connect_backend(&target, client_handshake).await {
                                    Ok(mut new_backend) => {
                                        match self.login_success_backend(&mut new_backend).await {
                                            Ok(_) => {
                                                *backend = new_backend;
                                                self.current_server = target.clone();
                                                let _ = self.send_chat_message(&format!("§aSuccessfully connected to '{}'!", target)).await;
                                                continue;
                                            }
                                            Err(err) => {
                                                let _ = self.send_chat_message(&format!("§cLogin to '{}' failed: {}", target, err)).await;
                                                continue;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        let _ = self.send_chat_message(&format!("§cCould not connect to '{}': {}", target, err)).await;
                                        continue;
                                    }
                                }
                            }
                            ProxyCommand::Glist => {
                                let count = self.online_players.load(Ordering::Relaxed);
                                let msg = format!("§b[Noncity] §7Total online players: §a{}", count);
                                let _ = self.send_chat_message(&msg).await;
                                continue;
                            }
                            ProxyCommand::Noncity => {
                                let msg = "§bNoncity §7v0.1.0";
                                let _ = self.send_chat_message(msg).await;
                                continue;
                            }
                            ProxyCommand::PluginCustom(ref full_cmd) => {
                                if let Some(reply) = self.plugins.on_command(&self.player_name, full_cmd) {
                                    let _ = self.send_chat_message(&reply).await;
                                    continue;
                                }
                            }
                        }
                    }

                    let mut packet = packet;
                    if self.config.cross_version && self.protocol_version != self.config.backend_protocol {
                        packet.id = translate_serverbound_id(packet.id, self.protocol_version, self.config.backend_protocol);
                    }
                    backend.write_packet(packet).await?;
                    backend.flush().await?;
                }
                backend_res = backend.read_packet() => {
                    let mut packet = backend_res?;
                    if self.config.cross_version && self.protocol_version != self.config.backend_protocol {
                        packet.id = translate_clientbound_id(packet.id, self.config.backend_protocol, self.protocol_version);
                    }
                    self.client.write_packet(packet).await?;
                    self.client.flush().await?;
                }
            }
        }
    }
}
