use rand::rngs::OsRng;
use rand::RngCore;
use rsa::pkcs8::EncodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthManager {
    private_key: Arc<RsaPrivateKey>,
    public_key_der: Arc<Vec<u8>>,
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct MojangProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MojangProfile {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub properties: Vec<MojangProfileProperty>,
}

impl AuthManager {
    pub fn new() -> anyhow::Result<Self> {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 1024)?;
        let public_key = RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_public_key_der()?.as_bytes().to_vec();

        Ok(Self {
            private_key: Arc::new(private_key),
            public_key_der: Arc::new(public_key_der),
            http_client: reqwest::Client::builder().build()?,
        })
    }

    pub fn public_key_der(&self) -> &[u8] {
        &self.public_key_der
    }

    pub fn generate_verify_token(&self) -> [u8; 4] {
        let mut token = [0u8; 4];
        OsRng.fill_bytes(&mut token);
        token
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.private_key
            .decrypt(Pkcs1v15Encrypt, ciphertext)
            .map_err(|e| anyhow::anyhow!("rsa decryption error: {e}"))
    }

    pub async fn verify_mojang(
        &self,
        username: &str,
        server_hash: &str,
        ip: Option<&str>,
    ) -> anyhow::Result<MojangProfile> {
        let mut url = format!(
            "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
            username, server_hash
        );
        if let Some(ip) = ip {
            url.push_str(&format!("&ip={}", ip));
        }

        let resp = self.http_client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("mojang authentication failed: status {}", resp.status());
        }

        let profile: MojangProfile = resp.json().await?;
        Ok(profile)
    }
}

pub fn compute_server_hash(
    server_id: &str,
    shared_secret: &[u8],
    public_key_der: &[u8],
) -> String {
    let mut hasher = Sha1::new();
    hasher.update(server_id.as_bytes());
    hasher.update(shared_secret);
    hasher.update(public_key_der);
    let digest = hasher.finalize();

    minecraft_hex_digest(&digest)
}

fn minecraft_hex_digest(bytes: &[u8]) -> String {
    let is_negative = (bytes[0] & 0x80) != 0;
    if is_negative {
        let mut twos = bytes.to_vec();
        let mut carry = true;
        for byte in twos.iter_mut().rev() {
            *byte = !*byte;
            if carry {
                let (val, next_carry) = byte.overflowing_add(1);
                *byte = val;
                carry = next_carry;
            }
        }
        let hex = hex_encode_trimmed(&twos);
        format!("-{}", hex)
    } else {
        hex_encode_trimmed(bytes)
    }
}

fn hex_encode_trimmed(bytes: &[u8]) -> String {
    let mut hex = String::new();
    let mut leading_zero = true;
    for byte in bytes {
        if leading_zero && *byte == 0 {
            continue;
        }
        if leading_zero {
            hex.push_str(&format!("{:x}", byte));
            leading_zero = false;
        } else {
            hex.push_str(&format!("{:02x}", byte));
        }
    }
    if hex.is_empty() {
        "0".to_string()
    } else {
        hex
    }
}
