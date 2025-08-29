use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn hmac_sha256(message: &str, secret: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}

pub fn hmac_sha256_base64(message: &str, secret: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}
