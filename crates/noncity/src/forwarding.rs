use bytes::Bytes;
use hmac::{Hmac, Mac};
use noncity_proto::io::{write_string, write_uuid};
use noncity_proto::varint::VarInt;
use sha2::Sha256;
use std::net::SocketAddr;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub const VELOCITY_FORWARDING_CHANNEL: &str = "velocity:player_info";
pub const VELOCITY_FORWARDING_VERSION: i32 = 1;

pub fn create_velocity_forwarding_data(
    secret: &str,
    remote_addr: SocketAddr,
    player_uuid: Uuid,
    player_name: &str,
) -> anyhow::Result<Bytes> {
    let mut payload = Vec::new();

    VarInt(VELOCITY_FORWARDING_VERSION).encode(&mut payload);
    write_string(&mut payload, &remote_addr.ip().to_string());
    write_uuid(&mut payload, &player_uuid);
    write_string(&mut payload, player_name);
    VarInt(0).encode(&mut payload);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(&payload);
    let signature = mac.finalize().into_bytes();

    let mut full_response = Vec::with_capacity(32 + payload.len());
    full_response.extend_from_slice(&signature);
    full_response.extend_from_slice(&payload);

    Ok(Bytes::from(full_response))
}
