use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::{RngCore, thread_rng};
use sha1::{Digest, Sha1};

use crate::error::AppError;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 生成指定长度的随机字节。
pub fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; length];
    thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// 生成 16 字节用户口令盐的 hex 表示。
pub fn generate_salt() -> String {
    hex(&random_bytes(16))
}

/// 生成 16 字节随机 ID 的 hex 表示。
pub fn random_id() -> String {
    hex(&random_bytes(16))
}

/// 后端存储口令：SHA1(盐 + 前端 SHA1)。
pub fn hash_password(salt: &str, password_sha1: &str) -> String {
    hex(&Sha1::digest(format!("{salt}{password_sha1}").as_bytes()))
}

/// 常数时间比较口令摘要，避免短路泄漏长度差异。
pub fn verify_password(salt: &str, password_sha1: &str, expected_hash: &str) -> bool {
    let actual = hash_password(salt, password_sha1);
    actual.as_bytes() == expected_hash.as_bytes()
}

/// AES-256-GCM 加密上游令牌，输出 nonce + 密文。
pub fn encrypt_secret(master_key: &[u8], plaintext: &str) -> Result<String, AppError> {
    let cipher = aes_cipher(master_key)?;
    let nonce_bytes = random_bytes(12);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| AppError::internal("secret encryption failed"))?;

    Ok(format!("{}{}", hex(&nonce_bytes), hex(&ciphertext)))
}

/// 解密 AES-256-GCM 上游令牌。
#[allow(dead_code)]
pub fn decrypt_secret(master_key: &[u8], ciphertext: &str) -> Result<String, AppError> {
    let bytes = unhex(ciphertext)?;
    if bytes.len() < 12 {
        return Err(AppError::internal("encrypted secret is malformed"));
    }

    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let cipher = aes_cipher(master_key)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| AppError::internal("secret decryption failed"))?;

    String::from_utf8(plaintext).map_err(|_| AppError::internal("encrypted secret is malformed"))
}

fn aes_cipher(master_key: &[u8]) -> Result<Aes256Gcm, AppError> {
    if master_key.len() != 32 {
        return Err(AppError::internal("master key must contain 32 bytes"));
    }

    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key)))
}

fn unhex(value: &str) -> Result<Vec<u8>, AppError> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::internal("encrypted secret is malformed"));
    }

    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| AppError::internal("encrypted secret is malformed"))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_uses_random_salt() {
        let first = hash_password("salt", "password");
        let second = hash_password("salt", "password");

        assert_eq!(first, second);
        assert_eq!(first.len(), 40);
        assert!(verify_password("salt", "password", &first));
        assert!(!verify_password("salt", "wrong", &first));
    }

    #[test]
    fn secret_encryption_uses_unique_nonce() {
        let key = b"0123456789abcdef0123456789abcdef";
        let first = encrypt_secret(key, "upstream-token").unwrap();
        let second = encrypt_secret(key, "upstream-token").unwrap();

        assert_ne!(first, second);
        assert_eq!(decrypt_secret(key, &first).unwrap(), "upstream-token");
    }
}
