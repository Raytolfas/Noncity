use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtoError {
    #[error("buffer underflow")]
    Underflow,
    #[error("varint too wide")]
    VarIntTooWide,
    #[error("varlong too wide")]
    VarLongTooWide,
    #[error("string too long: {0} > {1}")]
    StringTooLong(usize, usize),
    #[error("invalid utf-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("unknown packet id {0:#x} in state {1:?}")]
    UnknownPacket(i32, crate::State),
    #[error("invalid enum discriminant")]
    InvalidDiscriminant,
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
