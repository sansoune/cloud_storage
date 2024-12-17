use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use crate::{Result, StorageError};

pub struct EncryptionConfig {
    key: [u8; 32],
    enabled: bool,
}

impl EncryptionConfig {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            enabled: true,
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(data.to_vec());
        }

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(b"somedumbshit");

        cipher.encrypt(nonce, data).map_err(|err| crate::AppError::Storage(StorageError::Storage(format!("Encryption Error: {}", err))))
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(data.to_vec());
        }

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(b"somedumbshit");

        cipher.decrypt(nonce, data)
            .map_err(|e| crate::AppError::Storage(StorageError::Storage(format!("Decryption error: {}", e))))
    }
}