#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};
    use noncity_net::cipher::CryptState;
    use noncity_net::codec::MinecraftCodec;
    use noncity_net::compression::{compress, decompress};
    use noncity_net::packet::RawPacket;
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn test_crypt_state_roundtrip() {
        let key = [0x5au8; 16];
        let mut enc_state = CryptState::new(&key);
        let mut dec_state = CryptState::new(&key);

        let data = b"Minecraft proxy packets over TLS/CFB8";
        let mut buffer = data.to_vec();

        enc_state.encrypt(&mut buffer);
        assert_ne!(&buffer[..], data);

        dec_state.decrypt(&mut buffer);
        assert_eq!(&buffer[..], data);
    }

    #[test]
    fn test_compression_roundtrip() {
        let data = b"repeat_data_string_for_compression_testing_1234567890_repeat_repeat";
        let compressed = compress(data).unwrap();
        assert!(!compressed.is_empty());

        let decompressed = decompress(&compressed, data.len()).unwrap();
        assert_eq!(&decompressed[..], data);
    }

    #[test]
    fn test_codec_compressed_roundtrip() {
        let mut codec = MinecraftCodec::new();
        codec.set_compression_threshold(32);

        let packet = RawPacket::new(0x02, Bytes::from(vec![42u8; 128]));
        let mut buf = BytesMut::new();

        codec.encode(packet.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().expect("packet expected");

        assert_eq!(decoded.id, packet.id);
        assert_eq!(decoded.payload, packet.payload);
    }
}
