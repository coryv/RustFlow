use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
    Aes256Gcm, Nonce 
};
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// We need a master key. For CLI, we can use an env var or a fixed key for now (POC).
// In production, this should be securely managed.
// For this implementation, we will expect RUSTFLOW_MASTER_KEY env var (32 bytes hex), 
// or generate one if missing (but that means we can't decrypt later if we restart).
// Better: Store a key file locally if not present?
// Or just ask user to provide it.
// Let's use a default fixed key for "dev" mode if env is missing, but warn.

const DEFAULT_DEV_KEY: &[u8; 32] = b"01234567890123456789012345678901"; // 32 bytes

fn get_master_key() -> [u8; 32] {
    if let Ok(key_hex) = std::env::var("RUSTFLOW_MASTER_KEY") {
        if let Ok(bytes) = hex::decode(key_hex) {
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return key;
            }
        }
    }
    // Fallback for dev
    *DEFAULT_DEV_KEY
}

pub fn encrypt(data: &str) -> Result<String> {
    let key = get_master_key();
    let cipher = Aes256Gcm::new(&key.into());
    
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits; unique per message

    let ciphertext = cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| anyhow!("Encryption failure: {:?}", e))?;

    // Prepend nonce to ciphertext
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    Ok(BASE64.encode(combined))
}

pub fn decrypt(encrypted_data: &str) -> Result<String> {
    let key = get_master_key();
    let cipher = Aes256Gcm::new(&key.into());

    let decoded = BASE64.decode(encrypted_data)?;
    if decoded.len() < 12 {
        return Err(anyhow!("Invalid encrypted data length"));
    }

    let (nonce_bytes, ciphertext) = decoded.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failure: {:?}", e))?;

    Ok(String::from_utf8(plaintext)?)
}
