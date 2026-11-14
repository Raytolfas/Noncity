use crate::error::ProtoError;
use crate::io::{read_string, read_uuid, write_string, write_uuid, PacketRead, PacketWrite};
use crate::varint::VarInt;
use bytes::{Buf, BufMut, Bytes};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginStart {
    pub username: String,
    pub player_uuid: Option<Uuid>,
}

impl PacketRead for LoginStart {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let username = read_string(buf, 16)?;
        let player_uuid = if buf.remaining() >= 16 {
            Some(read_uuid(buf)?)
        } else if buf.remaining() == 1 {
            let has_sig = buf.get_u8() != 0;
            if has_sig && buf.remaining() >= 16 {
                Some(read_uuid(buf)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            username,
            player_uuid,
        })
    }
}

impl PacketWrite for LoginStart {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_string(buf, &self.username);
        if let Some(uuid) = &self.player_uuid {
            write_uuid(buf, uuid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
    pub should_authenticate: Option<bool>,
}

impl PacketRead for EncryptionRequest {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let server_id = read_string(buf, 20)?;
        let pk_len = VarInt::decode(buf)?.0 as usize;
        if buf.remaining() < pk_len {
            return Err(ProtoError::Underflow);
        }
        let mut public_key = vec![0u8; pk_len];
        buf.copy_to_slice(&mut public_key);

        let vt_len = VarInt::decode(buf)?.0 as usize;
        if buf.remaining() < vt_len {
            return Err(ProtoError::Underflow);
        }
        let mut verify_token = vec![0u8; vt_len];
        buf.copy_to_slice(&mut verify_token);

        let should_authenticate = if buf.remaining() > 0 {
            Some(buf.get_u8() != 0)
        } else {
            None
        };

        Ok(Self {
            server_id,
            public_key,
            verify_token,
            should_authenticate,
        })
    }
}

impl PacketWrite for EncryptionRequest {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_string(buf, &self.server_id);
        VarInt(self.public_key.len() as i32).encode(buf);
        buf.put_slice(&self.public_key);
        VarInt(self.verify_token.len() as i32).encode(buf);
        buf.put_slice(&self.verify_token);
        if let Some(auth) = self.should_authenticate {
            buf.put_u8(u8::from(auth));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl PacketRead for EncryptionResponse {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let ss_len = VarInt::decode(buf)?.0 as usize;
        if buf.remaining() < ss_len {
            return Err(ProtoError::Underflow);
        }
        let mut shared_secret = vec![0u8; ss_len];
        buf.copy_to_slice(&mut shared_secret);

        let vt_len = VarInt::decode(buf)?.0 as usize;
        if buf.remaining() < vt_len {
            return Err(ProtoError::Underflow);
        }
        let mut verify_token = vec![0u8; vt_len];
        buf.copy_to_slice(&mut verify_token);

        Ok(Self {
            shared_secret,
            verify_token,
        })
    }
}

impl PacketWrite for EncryptionResponse {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        VarInt(self.shared_secret.len() as i32).encode(buf);
        buf.put_slice(&self.shared_secret);
        VarInt(self.verify_token.len() as i32).encode(buf);
        buf.put_slice(&self.verify_token);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCompression {
    pub threshold: VarInt,
}

impl PacketRead for SetCompression {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let threshold = VarInt::decode(buf)?;
        Ok(Self { threshold })
    }
}

impl PacketWrite for SetCompression {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        self.threshold.encode(buf);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

impl PacketRead for LoginSuccess {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let uuid = read_uuid(buf)?;
        let username = read_string(buf, 16)?;
        let _num_properties = VarInt::decode(buf)?;
        Ok(Self { uuid, username })
    }
}

impl PacketWrite for LoginSuccess {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_uuid(buf, &self.uuid);
        write_string(buf, &self.username);
        VarInt(0).encode(buf);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginDisconnect {
    pub reason: String,
}

impl PacketRead for LoginDisconnect {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let reason = read_string(buf, 32767)?;
        Ok(Self { reason })
    }
}

impl PacketWrite for LoginDisconnect {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_string(buf, &self.reason);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPluginRequest {
    pub message_id: VarInt,
    pub channel: String,
    pub data: Bytes,
}

impl PacketRead for LoginPluginRequest {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let message_id = VarInt::decode(buf)?;
        let channel = read_string(buf, 32767)?;
        let mut data_vec = vec![0u8; buf.remaining()];
        buf.copy_to_slice(&mut data_vec);
        Ok(Self {
            message_id,
            channel,
            data: Bytes::from(data_vec),
        })
    }
}

impl PacketWrite for LoginPluginRequest {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        self.message_id.encode(buf);
        write_string(buf, &self.channel);
        buf.put_slice(&self.data);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPluginResponse {
    pub message_id: VarInt,
    pub successful: bool,
    pub data: Option<Bytes>,
}

impl PacketRead for LoginPluginResponse {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let message_id = VarInt::decode(buf)?;
        if !buf.has_remaining() {
            return Err(ProtoError::Underflow);
        }
        let successful = buf.get_u8() != 0;
        let data = if successful && buf.has_remaining() {
            let mut data_vec = vec![0u8; buf.remaining()];
            buf.copy_to_slice(&mut data_vec);
            Some(Bytes::from(data_vec))
        } else {
            None
        };
        Ok(Self {
            message_id,
            successful,
            data,
        })
    }
}

impl PacketWrite for LoginPluginResponse {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        self.message_id.encode(buf);
        buf.put_u8(if self.successful { 1 } else { 0 });
        if let Some(data) = &self.data {
            buf.put_slice(data);
        }
        Ok(())
    }
}
