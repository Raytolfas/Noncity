use crate::codec::MinecraftCodec;
use crate::error::NetError;
use crate::packet::RawPacket;
use crate::stream::CipherStream;
use futures_util::{SinkExt, StreamExt};
use noncity_proto::io::PacketWrite;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

pub struct Connection {
    framed: Framed<CipherStream<TcpStream>, MinecraftCodec>,
    peer_addr: SocketAddr,
}

impl Connection {
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            framed: Framed::new(CipherStream::new(stream), MinecraftCodec::default()),
            peer_addr,
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn set_compression(&mut self, threshold: i32) {
        self.framed.codec_mut().set_compression_threshold(threshold);
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.framed.get_mut().enable_crypto(key);
    }

    pub async fn read_packet(&mut self) -> Result<RawPacket, NetError> {
        match self.framed.next().await {
            Some(res) => res,
            None => Err(NetError::UnexpectedEof),
        }
    }

    pub async fn write_packet(&mut self, packet: RawPacket) -> Result<(), NetError> {
        self.framed.send(packet).await
    }

    pub async fn write<P: PacketWrite>(&mut self, id: i32, packet: &P) -> Result<(), NetError> {
        let raw = RawPacket::serialize(id, packet)?;
        self.write_packet(raw).await
    }

    pub async fn flush(&mut self) -> Result<(), NetError> {
        self.framed.flush().await
    }
}
