use crate::error::NetError;
use crate::packet::RawPacket;
use bytes::{Buf, Bytes, BytesMut};
use noncity_proto::varint::VarInt;
use tokio_util::codec::{Decoder, Encoder};

pub const MAX_PACKET_SIZE: usize = 2097152;

#[derive(Default)]
pub struct MinecraftCodec;

impl Decoder for MinecraftCodec {
    type Item = RawPacket;
    type Error = NetError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut temp = &src[..];
        let length = match VarInt::decode(&mut temp) {
            Ok(v) => v.0,
            Err(noncity_proto::ProtoError::Underflow) => return Ok(None),
            Err(e) => return Err(NetError::Proto(e)),
        };

        if length <= 0 {
            return Err(NetError::Proto(noncity_proto::ProtoError::Underflow));
        }

        let length = length as usize;
        if length > MAX_PACKET_SIZE {
            return Err(NetError::PacketTooLarge(length));
        }

        let varint_len = src.len() - temp.len();
        if src.len() < varint_len + length {
            src.reserve((varint_len + length) - src.len());
            return Ok(None);
        }

        src.advance(varint_len);
        let mut packet_data = src.split_to(length);

        let packet_id = VarInt::decode(&mut packet_data)
            .map_err(NetError::Proto)?
            .0;
        let payload: Bytes = packet_data.freeze();

        Ok(Some(RawPacket::new(packet_id, payload)))
    }
}

impl Encoder<RawPacket> for MinecraftCodec {
    type Error = NetError;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let id_varint = VarInt(item.id);
        let content_len = id_varint.encoded_len() + item.payload.len();
        let len_varint = VarInt(content_len as i32);

        dst.reserve(len_varint.encoded_len() + content_len);
        len_varint.encode(dst);
        id_varint.encode(dst);
        dst.extend_from_slice(&item.payload);

        Ok(())
    }
}
