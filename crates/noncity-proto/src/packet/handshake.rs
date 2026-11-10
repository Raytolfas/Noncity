use crate::error::ProtoError;
use crate::io::{read_string, write_string, PacketRead, PacketWrite};
use crate::varint::VarInt;
use bytes::{Buf, BufMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
    Transfer = 3,
}

impl TryFrom<i32> for HandshakeNextState {
    type Error = ProtoError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        match val {
            1 => Ok(Self::Status),
            2 => Ok(Self::Login),
            3 => Ok(Self::Transfer),
            _ => Err(ProtoError::InvalidDiscriminant),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakePacket {
    pub protocol_version: VarInt,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

impl PacketRead for HandshakePacket {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let protocol_version = VarInt::decode(buf)?;
        let server_address = read_string(buf, 255)?;
        if buf.remaining() < 2 {
            return Err(ProtoError::Underflow);
        }
        let server_port = buf.get_u16();
        let next_state_raw = VarInt::decode(buf)?.0;
        let next_state = HandshakeNextState::try_from(next_state_raw)?;

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

impl PacketWrite for HandshakePacket {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        self.protocol_version.encode(buf);
        write_string(buf, &self.server_address);
        buf.put_u16(self.server_port);
        VarInt(self.next_state as i32).encode(buf);
        Ok(())
    }
}
