#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use noncity_proto::io::{read_string, write_string, PacketRead, PacketWrite};
    use noncity_proto::packet::handshake::{HandshakeNextState, HandshakePacket};
    use noncity_proto::varint::{VarInt, VarLong};

    #[test]
    fn test_varint_roundtrip() {
        let test_cases = [0, 1, 127, 128, 255, 256, 2147483647, -1, -2147483648];
        for &val in &test_cases {
            let mut buf = Vec::new();
            VarInt(val).encode(&mut buf);
            assert_eq!(buf.len(), VarInt(val).encoded_len());

            let mut slice = buf.as_slice();
            let decoded = VarInt::decode(&mut slice).unwrap();
            assert_eq!(decoded.0, val);
            assert_eq!(slice.len(), 0);
        }
    }

    #[test]
    fn test_varlong_roundtrip() {
        let test_cases = [
            0,
            1,
            i64::MAX,
            -1,
            i64::MIN,
            1234567890123456789,
            -1234567890123456789,
        ];
        for &val in &test_cases {
            let mut buf = Vec::new();
            VarLong(val).encode(&mut buf);

            let mut slice = buf.as_slice();
            let decoded = VarLong::decode(&mut slice).unwrap();
            assert_eq!(decoded.0, val);
            assert_eq!(slice.len(), 0);
        }
    }

    #[test]
    fn test_string_io() {
        let original = "Hello Minecraft Noncity!";
        let mut buf = BytesMut::new();
        write_string(&mut buf, original);

        let mut slice = &buf[..];
        let read = read_string(&mut slice, 32767).unwrap();
        assert_eq!(read, original);
    }

    #[test]
    fn test_handshake_roundtrip() {
        let packet = HandshakePacket {
            protocol_version: VarInt(765),
            server_address: "localhost".to_string(),
            server_port: 25565,
            next_state: HandshakeNextState::Status,
        };

        let mut buf = Vec::new();
        packet.write(&mut buf).unwrap();

        let mut slice = buf.as_slice();
        let decoded = HandshakePacket::read(&mut slice).unwrap();
        assert_eq!(decoded, packet);
    }
}
