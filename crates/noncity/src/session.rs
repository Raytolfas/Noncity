use crate::auth::{compute_server_hash, AuthManager, MojangProfileProperty};
use crate::command::{extract_proxy_command, handle_tab_complete, make_chat_json, ProxyCommand};
use crate::config::{Config, ForwardingMode};
use crate::forwarding::{create_velocity_forwarding_data, VELOCITY_FORWARDING_CHANNEL};
use crate::plugin::PluginManager;
use noncity_net::Connection;
use noncity_proto::packet::handshake::{HandshakeNextState, HandshakePacket};
use noncity_proto::packet::login::{
    EncryptionRequest, EncryptionResponse, LoginDisconnect, LoginPluginRequest,
    LoginPluginResponse, LoginStart, LoginSuccess, SetCompression,
};
use noncity_proto::packet::play::{
    command_suggestions_packet_id, system_chat_packet_id, tablist_packet_id, SystemChatPacket,
    TabListHeaderFooter,
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

        let status = ServerStatus {
            version: StatusVersion {
                name: "Noncity".to_string(),
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

    async fn handle_login(&mut self, client_handshake: HandshakePacket) -> anyhow::Result<()> {
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
            let enc_request = EncryptionRequest {
                server_id: "".to_string(),
                public_key: self.auth.public_key_der().to_vec(),
                verify_token: verify_token.to_vec(),
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

        self.login_success_backend(&mut backend).await?;
        self.current_server = default_target;

        let login_success = LoginSuccess {
            uuid: self.player_uuid,
            username: self.player_name.clone(),
        };
        self.client.write(0x02, &login_success).await?;
        self.client.flush().await?;

        if self.protocol_version >= 764 {
            let ack = self.client.read_packet().await?;
            backend.write_packet(ack).await?;
            backend.flush().await?;
        }

        self.send_tablist_header_footer().await?;

        self.online_players.fetch_add(1, Ordering::Relaxed);
        info!(player = %self.player_name, server = %self.current_server, "player joined proxy");

        let pipe_res = self.play_loop(&mut backend, &client_handshake).await;
        self.online_players.fetch_sub(1, Ordering::Relaxed);

        info!(player = %self.player_name, "player disconnected");
        pipe_res
    }

    async fn send_tablist_header_footer(&mut self) -> anyhow::Result<()> {
        let packet_id = tablist_packet_id(self.protocol_version);
        let tab = TabListHeaderFooter {
            header: make_chat_json(&self.config.tablist_header),
            footer: make_chat_json(&self.config.tablist_footer),
        };
        let _ = self.client.write(packet_id, &tab).await;
        let _ = self.client.flush().await;
        Ok(())
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

    async fn login_success_backend(&mut self, backend: &mut Connection) -> anyhow::Result<()> {
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
                    break;
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
        Ok(())
    }

    async fn send_chat_message(&mut self, message: &str) -> anyhow::Result<()> {
        let packet_id = system_chat_packet_id(self.protocol_version);
        let chat = SystemChatPacket {
            content: make_chat_json(message),
            overlay: false,
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

                    if let Some(tab_resp) = handle_tab_complete(&packet, &server_names) {
                        let packet_id = command_suggestions_packet_id(self.protocol_version);
                        let _ = self.client.write(packet_id, &tab_resp).await;
                        let _ = self.client.flush().await;
                        continue;
                    }

                    if let Some(cmd) = extract_proxy_command(&packet) {
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
                                            Ok(()) => {
                                                *backend = new_backend;
                                                self.current_server = target.clone();
                                                let _ = self.send_chat_message(&format!("§aSuccessfully connected to '{}'!", target)).await;
                                                let _ = self.send_tablist_header_footer().await;
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
                                let msg = "§bNoncity §7v0.1.0 - High performance Rust Minecraft proxy (tokio core)";
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

                    backend.write_packet(packet).await?;
                    backend.flush().await?;
                }
                backend_res = backend.read_packet() => {
                    let packet = backend_res?;
                    self.client.write_packet(packet).await?;
                    self.client.flush().await?;
                }
            }
        }
    }
}
