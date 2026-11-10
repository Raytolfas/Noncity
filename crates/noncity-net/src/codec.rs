use crate::compression::{compress, decompress};
use crate::error::NetError;
use crate::packet::RawPacket;
use bytes::{Buf, Bytes, BytesMut};
use noncity_proto::varint::VarInt;
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};

pub const MAX_PACKET_SIZE: usize = 2097152;

#[derive(Default, Clone)]
pub struct MinecraftCodec {
    compression_threshold: Option<i32>,
}

impl MinecraftCodec {
    pub fn new() -> Self {
        Self {
            compression_threshold: None,
        }
    }

    pub fn set_compression_threshold(&mut self, threshold: i32) {
        if threshold >= 0 {
            self.compression_threshold = Some(threshold);
        } else {
            self.compression_threshold = None;
        }
    }

    pub fn compression_threshold(&self) -> Option<i32> {
        self.compression_threshold
    }
}

impl Decoder for MinecraftCodec {
    type Item = RawPacket;
    type Error = NetError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut temp = &src[..];
        let packet_len = match VarInt::decode(&mut temp) {
            Ok(v) => v.0,
            Err(noncity_proto::ProtoError::Underflow) => return Ok(None),
            Err(e) => return Err(NetError::Proto(e)),
        };

        if packet_len <= 0 {
            return Err(NetError::Proto(noncity_proto::ProtoError::Underflow));
        }

        let packet_len = packet_len as usize;
        if packet_len > MAX_PACKET_SIZE {
            return Err(NetError::PacketTooLarge(packet_len));
        }

        let varint_len = src.len() - temp.len();
        if src.len() < varint_len + packet_len {
            src.reserve((varint_len + packet_len) - src.len());
            return Ok(None);
        }

        src.advance(varint_len);
        let mut packet_data = src.split_to(packet_len);

        if self.compression_threshold.is_some() {
            let data_len = VarInt::decode(&mut packet_data)
                .map_err(NetError::Proto)?
                .0;

            if data_len < 0 {
                return Err(NetError::Proto(noncity_proto::ProtoError::Underflow));
            }

            if data_len == 0 {
                let packet_id = VarInt::decode(&mut packet_data)
                    .map_err(NetError::Proto)?
                    .0;
                let payload = packet_data.freeze();
                Ok(Some(RawPacket::new(packet_id, payload)))
            } else {
                let decompressed = decompress(&packet_data, data_len as usize)?;
                let mut cursor = Cursor::new(decompressed);
                let packet_id = VarInt::decode(&mut cursor).map_err(NetError::Proto)?.0;
                let pos = cursor.position() as usize;
                let payload = Bytes::from(cursor.into_inner()[pos..].to_vec());
                Ok(Some(RawPacket::new(packet_id, payload)))
            }
        } else {
            let packet_id = VarInt::decode(&mut packet_data)
                .map_err(NetError::Proto)?
                .0;
            let payload = packet_data.freeze();
            Ok(Some(RawPacket::new(packet_id, payload)))
        }
    }
}

impl Encoder<RawPacket> for MinecraftCodec {
    type Error = NetError;

    fn encode(&mut self, item: RawPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let id_varint = VarInt(item.id);

        if let Some(threshold) = self.compression_threshold {
            let mut uncompressed = Vec::with_capacity(id_varint.encoded_len() + item.payload.len());
            id_varint.encode(&mut uncompressed);
            uncompressed.extend_from_slice(&item.payload);

            let uncompressed_len = uncompressed.len();

            if (uncompressed_len as i32) < threshold {
                let data_len_varint = VarInt(0);
                let packet_len = data_len_varint.encoded_len() + uncompressed_len;
                let packet_len_varint = VarInt(packet_len as i32);

                dst.reserve(packet_len_varint.encoded_len() + packet_len);
                packet_len_varint.encode(dst);
                data_len_varint.encode(dst);
                dst.extend_from_slice(&uncompressed);
            } else {
                let compressed = compress(&uncompressed)?;
                let data_len_varint = VarInt(uncompressed_len as i32);
                let packet_len = data_len_varint.encoded_len() + compressed.len();
                let packet_len_varint = VarInt(packet_len as i32);

                dst.reserve(packet_len_varint.encoded_len() + packet_len);
                packet_len_varint.encode(dst);
                data_len_varint.encode(dst);
                dst.extend_from_slice(&compressed);
            }
        } else {
            let content_len = id_varint.encoded_len() + item.payload.len();
            let len_varint = VarInt(content_len as i32);

            dst.reserve(len_varint.encoded_len() + content_len);
            len_varint.encode(dst);
            id_varint.encode(dst);
            dst.extend_from_slice(&item.payload);
        }

        Ok(())
    }
}
