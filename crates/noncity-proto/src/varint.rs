use crate::error::ProtoError;
use bytes::{Buf, BufMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct VarInt(pub i32);

impl VarInt {
    pub fn decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut value = 0i32;
        let mut shift = 0;

        for _ in 0..5 {
            if !buf.has_remaining() {
                return Err(ProtoError::Underflow);
            }
            let byte = buf.get_u8();
            value |= ((byte & 0x7F) as i32) << shift;

            if (byte & 0x80) == 0 {
                return Ok(VarInt(value));
            }
            shift += 7;
        }

        Err(ProtoError::VarIntTooWide)
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        let mut val = self.0 as u32;
        loop {
            if (val & !0x7F) == 0 {
                buf.put_u8(val as u8);
                return;
            }
            buf.put_u8(((val & 0x7F) | 0x80) as u8);
            val >>= 7;
        }
    }

    pub fn encoded_len(self) -> usize {
        let mut val = self.0 as u32;
        let mut len = 0;
        loop {
            len += 1;
            if (val & !0x7F) == 0 {
                return len;
            }
            val >>= 7;
        }
    }
}

impl From<i32> for VarInt {
    fn from(v: i32) -> Self {
        VarInt(v)
    }
}

impl From<VarInt> for i32 {
    fn from(v: VarInt) -> Self {
        v.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct VarLong(pub i64);

impl VarLong {
    pub fn decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut value = 0i64;
        let mut shift = 0;

        for _ in 0..10 {
            if !buf.has_remaining() {
                return Err(ProtoError::Underflow);
            }
            let byte = buf.get_u8();
            value |= ((byte & 0x7F) as i64) << shift;

            if (byte & 0x80) == 0 {
                return Ok(VarLong(value));
            }
            shift += 7;
        }

        Err(ProtoError::VarLongTooWide)
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        let mut val = self.0 as u64;
        loop {
            if (val & !0x7F) == 0 {
                buf.put_u8(val as u8);
                return;
            }
            buf.put_u8(((val & 0x7F) | 0x80) as u8);
            val >>= 7;
        }
    }
}

impl From<i64> for VarLong {
    fn from(v: i64) -> Self {
        VarLong(v)
    }
}

impl From<VarLong> for i64 {
    fn from(v: VarLong) -> Self {
        v.0
    }
}
