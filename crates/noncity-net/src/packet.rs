use bytes::Bytes;
use noncity_proto::error::ProtoError;
use noncity_proto::io::{PacketRead, PacketWrite};
use noncity_proto::varint::VarInt;

#[derive(Debug, Clone)]
pub struct RawPacket {
    pub id: i32,
    pub payload: Bytes,
}

impl RawPacket {
    pub fn new(id: i32, payload: Bytes) -> Self {
        Self { id, payload }
    }

    pub fn deserialize<T: PacketRead>(&self) -> Result<T, ProtoError> {
        let mut slice = self.payload.as_ref();
        T::read(&mut slice)
    }

    pub fn serialize<T: PacketWrite>(id: i32, packet: &T) -> Result<Self, ProtoError> {
        let mut buf = Vec::new();
        packet.write(&mut buf)?;
        Ok(Self {
            id,
            payload: Bytes::from(buf),
        })
    }

    pub fn encoded_len(&self) -> usize {
        let id_varint = VarInt(self.id);
        let content_len = id_varint.encoded_len() + self.payload.len();
        let len_varint = VarInt(content_len as i32);
        len_varint.encoded_len() + content_len
    }
}
