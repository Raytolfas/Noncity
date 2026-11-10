#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use noncity_net::packet::RawPacket;
    use noncity_proto::io::write_string;
    use noncity_proto::uuid::offline_player_uuid;
    use uuid::Uuid;

    #[test]
    fn test_offline_uuid_computation() {
        let username = "Notch";
        let uuid = offline_player_uuid(username);
        let expected = Uuid::parse_str("b50ad385-829d-3141-a216-7e7d7539ba7f").unwrap();
        assert_eq!(uuid, expected);
        assert_eq!(uuid.get_version_num(), 3);
    }

    #[test]
    fn test_command_extraction_server() {
        let mut buf = Vec::new();
        write_string(&mut buf, "/server survival");
        let packet = RawPacket::new(0x04, Bytes::from(buf));

        let mut slice = packet.payload.as_ref();
        let text = noncity_proto::io::read_string(&mut slice, 256).unwrap();
        assert_eq!(text, "/server survival");
    }
}
