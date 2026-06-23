use aes::Aes128;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cbc::{
    cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit},
    Decryptor, Encryptor,
};
use rand::{distributions::Alphanumeric, Rng};
use thiserror::Error;

const KEY: &[u8; 16] = b"root%$#@!1234567";
const BLOCK_SIZE: usize = 16;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("待加密内容不能为空")]
    EmptyInput,
    #[error("密文格式错误：{0}")]
    InvalidData(&'static str),
    #[error("Base64 解码失败")]
    Base64(#[from] base64::DecodeError),
    #[error("解密结果不是 UTF-8 文本")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Reproduce the portal JavaScript format:
/// Base64(AES-128-CBC-ZeroPadding(UTF-8 text)) + 16-byte ASCII IV.
pub fn encrypt(text: &str) -> Result<String, CryptoError> {
    let iv: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(BLOCK_SIZE)
        .map(char::from)
        .collect();
    encrypt_with_iv(text, &iv)
}

pub(crate) fn encrypt_with_iv(text: &str, iv: &str) -> Result<String, CryptoError> {
    if text.is_empty() {
        return Err(CryptoError::EmptyInput);
    }
    if iv.len() != BLOCK_SIZE || !iv.is_ascii() {
        return Err(CryptoError::InvalidData("IV 必须是 16 字节 ASCII 文本"));
    }

    let plain = text.as_bytes();
    let padded_len = plain.len().div_ceil(BLOCK_SIZE) * BLOCK_SIZE;
    let mut buffer = vec![0_u8; padded_len];
    buffer[..plain.len()].copy_from_slice(plain);

    let ciphertext = Encryptor::<Aes128>::new_from_slices(KEY, iv.as_bytes())
        .map_err(|_| CryptoError::InvalidData("AES 密钥或 IV 长度错误"))?
        .encrypt_padded_mut::<NoPadding>(&mut buffer, padded_len)
        .map_err(|_| CryptoError::InvalidData("AES 加密失败"))?;

    Ok(format!("{}{}", STANDARD.encode(ciphertext), iv))
}

pub fn decrypt(value: &str) -> Result<String, CryptoError> {
    if value.len() <= BLOCK_SIZE || !value.is_ascii() {
        return Err(CryptoError::InvalidData("密文过短或包含非 ASCII 字符"));
    }

    let split_at = value.len() - BLOCK_SIZE;
    let (encoded, iv) = value.split_at(split_at);
    let mut ciphertext = STANDARD.decode(encoded)?;
    if ciphertext.is_empty() || ciphertext.len() % BLOCK_SIZE != 0 {
        return Err(CryptoError::InvalidData("AES 密文长度不是 16 的倍数"));
    }

    let plaintext = Decryptor::<Aes128>::new_from_slices(KEY, iv.as_bytes())
        .map_err(|_| CryptoError::InvalidData("AES 密钥或 IV 长度错误"))?
        .decrypt_padded_mut::<NoPadding>(&mut ciphertext)
        .map_err(|_| CryptoError::InvalidData("AES 解密失败"))?;

    let useful_len = plaintext
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |index| index + 1);
    Ok(String::from_utf8(plaintext[..useful_len].to_vec())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_aes_vector() {
        let encrypted = encrypt_with_iv("hello", "0123456789ABCDEF").unwrap();
        assert_eq!(encrypted, "843dwvBGs93Jld0gMUEdBg==0123456789ABCDEF");
        assert_eq!(decrypt(&encrypted).unwrap(), "hello");
    }

    #[test]
    fn exact_block_does_not_add_another_block() {
        let encrypted = encrypt_with_iv("1234567890ABCDEF", "FEDCBA9876543210").unwrap();
        let encoded = &encrypted[..encrypted.len() - BLOCK_SIZE];
        assert_eq!(STANDARD.decode(encoded).unwrap().len(), BLOCK_SIZE);
        assert_eq!(decrypt(&encrypted).unwrap(), "1234567890ABCDEF");
    }
}
