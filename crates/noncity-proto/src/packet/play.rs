use crate::error::ProtoError;
use crate::io::{read_string, write_string, PacketRead, PacketWrite};
use bytes::{Buf, BufMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChatPacket {
    pub content: String,
    pub overlay: bool,
}

impl PacketRead for SystemChatPacket {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let content = read_string(buf, 262144)?;
        let overlay = if buf.has_remaining() {
            buf.get_u8() != 0
        } else {
            false
        };
        Ok(Self { content, overlay })
    }
}

impl PacketWrite for SystemChatPacket {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        write_string(buf, &self.content);
        buf.put_u8(if self.overlay { 1 } else { 0 });
        Ok(())
    }
}

pub fn system_chat_packet_id(protocol_version: i32) -> i32 {
    if protocol_version >= 766 {
        0x6B
    } else if protocol_version >= 764 {
        0x69
    } else if protocol_version >= 763 {
        0x67
    } else if protocol_version >= 761 {
        0x60
    } else if protocol_version >= 759 {
        0x5F
    } else {
        0x0F
    }
}
