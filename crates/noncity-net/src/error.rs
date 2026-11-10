use noncity_proto::ProtoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("protocol error: {0}")]
    Proto(#[from] ProtoError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection closed unexpectedly")]
    UnexpectedEof,
    #[error("packet too large: {0} bytes")]
    PacketTooLarge(usize),
}
