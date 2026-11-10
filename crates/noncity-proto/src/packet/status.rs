use crate::error::ProtoError;
use crate::io::{read_string, write_string, PacketRead, PacketWrite};
use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRequest;

impl PacketRead for StatusRequest {
    fn read(_buf: &mut impl Buf) -> Result<Self, ProtoError> {
        Ok(Self)
    }
}

impl PacketWrite for StatusRequest {
    fn write(&self, _buf: &mut impl BufMut) -> Result<(), ProtoError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingPacket {
    pub payload: i64,
}

impl PacketRead for PingPacket {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 8 {
            return Err(ProtoError::Underflow);
        }
        Ok(Self {
            payload: buf.get_i64(),
        })
    }
}

impl PacketWrite for PingPacket {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        buf.put_i64(self.payload);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusResponse {
    pub json: String,
}

impl PacketRead for StatusResponse {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let json = read_string(buf, 32767)?;
        Ok(Self { json })
    }
}

impl PacketWrite for StatusResponse {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_string(buf, &self.json);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPlayerSample {
    pub name: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<StatusPlayerSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusDescription {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub version: StatusVersion,
    pub players: StatusPlayers,
    pub description: StatusDescription,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat", skip_serializing_if = "Option::is_none")]
    pub enforces_secure_chat: Option<bool>,
}
