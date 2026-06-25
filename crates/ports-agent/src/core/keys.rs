use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;

pub fn generate_and_store(key_path: &str, pub_path: &str) -> Result<String> {
    if let Some(parent) = Path::new(key_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let signing = SigningKey::generate(&mut OsRng);
    let public_b64 = STANDARD.encode(signing.verifying_key().to_bytes());

    std::fs::write(key_path, STANDARD.encode(signing.to_bytes()))
        .with_context(|| format!("writing {key_path}"))?;
    std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600))?;
    std::fs::write(pub_path, &public_b64).with_context(|| format!("writing {pub_path}"))?;

    Ok(public_b64)
}

pub fn public_key_b64(pub_path: &str) -> Result<String> {
    Ok(std::fs::read_to_string(pub_path)?.trim().to_string())
}

pub fn load_signing(key_path: &str) -> Result<SigningKey> {
    let raw = std::fs::read_to_string(key_path)?;
    let bytes = STANDARD.decode(raw.trim())?;
    let arr: [u8; SECRET_KEY_LENGTH] =
        bytes.try_into().map_err(|_| anyhow!("invalid signing key length"))?;
    Ok(SigningKey::from_bytes(&arr))
}

pub fn sign_b64(key_path: &str, message: &[u8]) -> Result<String> {
    let signing = load_signing(key_path)?;
    Ok(STANDARD.encode(signing.sign(message).to_bytes()))
}
