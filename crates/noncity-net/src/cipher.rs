use aes::Aes128;
use cfb8::cipher::generic_array::GenericArray;
use cfb8::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub type Aes128Cfb8Enc = cfb8::Encryptor<Aes128>;
pub type Aes128Cfb8Dec = cfb8::Decryptor<Aes128>;

pub struct CryptState {
    encryptor: Aes128Cfb8Enc,
    decryptor: Aes128Cfb8Dec,
}

impl CryptState {
    pub fn new(key: &[u8; 16]) -> Self {
        let encryptor = Aes128Cfb8Enc::new(key.into(), key.into());
        let decryptor = Aes128Cfb8Dec::new(key.into(), key.into());
        Self {
            encryptor,
            decryptor,
        }
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            let block = GenericArray::from_mut_slice(std::slice::from_mut(b));
            self.encryptor.encrypt_block_mut(block);
        }
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            let block = GenericArray::from_mut_slice(std::slice::from_mut(b));
            self.decryptor.decrypt_block_mut(block);
        }
    }
}
