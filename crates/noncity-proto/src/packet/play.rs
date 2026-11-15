use crate::error::ProtoError;
use crate::io::{read_string, write_string, PacketRead, PacketWrite};
use crate::varint::VarInt;
use bytes::{Buf, BufMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChatPacket {
    pub content: String,
    pub overlay: bool,
    pub protocol_version: i32,
}

impl PacketRead for SystemChatPacket {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let content = read_string(buf, 262144)?;
        let overlay = if buf.has_remaining() {
            buf.get_u8() != 0
        } else {
            false
        };
        Ok(Self {
            content,
            overlay,
            protocol_version: 0,
        })
    }
}

impl PacketWrite for SystemChatPacket {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        if self.protocol_version >= 765 {
            buf.put_u8(0x08);
            buf.put_u16(self.content.len() as u16);
            buf.put_slice(self.content.as_bytes());
        } else {
            write_string(buf, &self.content);
        }
        buf.put_u8(if self.overlay { 1 } else { 0 });
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabListHeaderFooter {
    pub header: String,
    pub footer: String,
    pub protocol_version: i32,
}

impl PacketRead for TabListHeaderFooter {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let header = read_string(buf, 262144)?;
        let footer = read_string(buf, 262144)?;
        Ok(Self {
            header,
            footer,
            protocol_version: 0,
        })
    }
}

impl PacketWrite for TabListHeaderFooter {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        if self.protocol_version >= 765 {
            buf.put_u8(0x08);
            buf.put_u16(self.header.len() as u16);
            buf.put_slice(self.header.as_bytes());
            buf.put_u8(0x08);
            buf.put_u16(self.footer.len() as u16);
            buf.put_slice(self.footer.as_bytes());
        } else {
            write_string(buf, &self.header);
            write_string(buf, &self.footer);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSuggestionMatch {
    pub value: String,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSuggestionsResponse {
    pub transaction_id: VarInt,
    pub start: VarInt,
    pub length: VarInt,
    pub matches: Vec<CommandSuggestionMatch>,
}

impl PacketWrite for CommandSuggestionsResponse {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError> {
        self.transaction_id.encode(buf);
        self.start.encode(buf);
        self.length.encode(buf);
        VarInt(self.matches.len() as i32).encode(buf);
        for m in &self.matches {
            write_string(buf, &m.value);
            if let Some(tip) = &m.tooltip {
                buf.put_u8(1);
                write_string(buf, tip);
            } else {
                buf.put_u8(0);
            }
        }
        Ok(())
    }
}

pub fn system_chat_packet_id(protocol_version: i32) -> i32 {
    if protocol_version >= 777 {
        0x7C
    } else if protocol_version >= 769 {
        0x73
    } else if protocol_version >= 768 {
        0x72
    } else if protocol_version >= 767 {
        0x6C
    } else if protocol_version >= 766 {
        0x6B
    } else if protocol_version >= 764 {
        0x69
    } else if protocol_version >= 763 {
        0x67
    } else if protocol_version >= 761 {
        0x60
    } else if protocol_version >= 759 {
        0x5F
    } else if protocol_version >= 755 {
        0x0F
    } else if protocol_version >= 735 {
        0x0E
    } else if protocol_version >= 573 {
        0x0F
    } else if protocol_version >= 393 {
        0x0E
    } else {
        0x02
    }
}

pub fn tablist_packet_id(protocol_version: i32) -> i32 {
    if protocol_version >= 777 {
        0x7D
    } else if protocol_version >= 766 {
        0x6E
    } else if protocol_version >= 764 {
        0x6C
    } else if protocol_version >= 763 {
        0x6A
    } else if protocol_version >= 761 {
        0x63
    } else if protocol_version >= 759 {
        0x62
    } else {
        0x54
    }
}

pub fn command_suggestions_packet_id(protocol_version: i32) -> i32 {
    if protocol_version >= 777 {
        0x0F
    } else if protocol_version >= 766 {
        0x11
    } else if protocol_version >= 764 {
        0x10
    } else if protocol_version >= 761 {
        0x0F
    } else {
        0x0E
    }
}

pub fn translate_clientbound_id(id: i32, from_proto: i32, to_proto: i32) -> i32 {
    if from_proto == to_proto {
        return id;
    }

    if from_proto >= 777 && to_proto < 777 {
        match id {
            0x7C => system_chat_packet_id(to_proto),
            0x7D => tablist_packet_id(to_proto),
            0x0F => command_suggestions_packet_id(to_proto),
            0x2B => {
                if to_proto >= 766 {
                    0x26
                } else if to_proto >= 764 {
                    0x24
                } else if to_proto >= 763 {
                    0x23
                } else if to_proto >= 759 {
                    0x21
                } else {
                    0x1F
                }
            }
            0x1F => {
                if to_proto >= 766 {
                    0x1D
                } else if to_proto >= 764 {
                    0x1B
                } else if to_proto >= 763 {
                    0x1A
                } else {
                    0x19
                }
            }
            0x32 => {
                if to_proto >= 766 {
                    0x2B
                } else if to_proto >= 764 {
                    0x29
                } else if to_proto >= 763 {
                    0x28
                } else if to_proto >= 759 {
                    0x26
                } else {
                    0x24
                }
            }
            _ => id,
        }
    } else if from_proto < 777 && to_proto >= 777 {
        if id == system_chat_packet_id(from_proto) {
            0x7C
        } else if id == tablist_packet_id(from_proto) {
            0x7D
        } else if id == command_suggestions_packet_id(from_proto) {
            0x0F
        } else {
            id
        }
    } else {
        id
    }
}

pub fn translate_serverbound_id(id: i32, from_proto: i32, to_proto: i32) -> i32 {
    if from_proto == to_proto {
        return id;
    }

    if from_proto < 777 && to_proto >= 777 {
        match id {
            0x18 => 0x18,
            _ => id,
        }
    } else {
        id
    }
}
