use md5::{Digest, Md5};
use uuid::Uuid;

pub fn offline_player_uuid(username: &str) -> Uuid {
    let input = format!("OfflinePlayer:{}", username);
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let mut hash = hasher.finalize();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    Uuid::from_bytes(hash.into())
}
