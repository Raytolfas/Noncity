pub mod error;
pub mod io;
pub mod packet;
pub mod uuid;
pub mod varint;

pub use error::ProtoError;
pub use io::{PacketRead, PacketWrite};
pub use uuid::offline_player_uuid;
pub use varint::{VarInt, VarLong};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}
