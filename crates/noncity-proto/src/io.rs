use crate::error::ProtoError;
use crate::varint::VarInt;
use bytes::{Buf, BufMut};
use uuid::Uuid;

pub trait PacketRead: Sized {
    fn read(buf: &mut impl Buf) -> Result<Self, ProtoError>;
}

pub trait PacketWrite {
    fn write(&self, buf: &mut impl BufMut) -> Result<(), ProtoError>;
}

pub fn read_string(buf: &mut impl Buf, max_len: usize) -> Result<String, ProtoError> {
    let len = VarInt::decode(buf)?.0;
    if len < 0 {
        return Err(ProtoError::Underflow);
    }
    let len = len as usize;
    if len > max_len * 4 {
        return Err(ProtoError::StringTooLong(len, max_len * 4));
    }
    if buf.remaining() < len {
        return Err(ProtoError::Underflow);
    }
    let mut bytes = vec![0u8; len];
    buf.copy_to_slice(&mut bytes);
    let s = String::from_utf8(bytes)?;
    if s.chars().count() > max_len {
        return Err(ProtoError::StringTooLong(s.chars().count(), max_len));
    }
    Ok(s)
}

pub fn write_string(buf: &mut impl BufMut, val: &str) {
    let bytes = val.as_bytes();
    VarInt(bytes.len() as i32).encode(buf);
    buf.put_slice(bytes);
}

pub fn read_uuid(buf: &mut impl Buf) -> Result<Uuid, ProtoError> {
    if buf.remaining() < 16 {
        return Err(ProtoError::Underflow);
    }
    let mut bytes = [0u8; 16];
    buf.copy_to_slice(&mut bytes);
    Ok(Uuid::from_bytes(bytes))
}

pub fn write_uuid(buf: &mut impl BufMut, val: &Uuid) {
    buf.put_slice(val.as_bytes());
}
