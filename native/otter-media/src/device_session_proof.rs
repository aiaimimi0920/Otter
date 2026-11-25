//! 设备 challenge 的身份校验与主干兼容签名；不发 HTTP、不自动重试/批准。
use crate::device_identity::{valid_field, Identity};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Identity,
    Challenge,
    Random,
    Sign,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Challenge {
    challenge_id: String,
    device_id: String,
    challenge: String,
}

pub fn create(identity: &Identity, body: &[u8]) -> Result<Vec<u8>, Error> {
    let mut nonce = [0u8; 24];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| Error::Random)?;
    create_with_nonce(identity, body, &URL_SAFE_NO_PAD.encode(nonce))
}
fn create_with_nonce(identity: &Identity, body: &[u8], nonce: &str) -> Result<Vec<u8>, Error> {
    if body.len() > 256 * 1024 {
        return Err(Error::Limit);
    }
    let challenge: Challenge = serde_json::from_slice(body).map_err(|_| Error::Json)?;
    let device = identity.device_id().ok_or(Error::Identity)?;
    if device != challenge.device_id {
        return Err(Error::Identity);
    }
    if !valid_field(&challenge.challenge_id, 160)
        || !valid_field(&challenge.challenge, 4096)
        || !valid_field(nonce, 128)
    {
        return Err(Error::Challenge);
    }
    let message = format!(
        "loom.device-session.v1\n{device}\n{}\n{}\n{nonce}",
        challenge.challenge_id, challenge.challenge
    );
    let signature = identity.sign(message.as_bytes()).map_err(|_| Error::Sign)?;
    serde_json::to_vec(
        &serde_json::json!({"deviceId":device,"challengeId":challenge.challenge_id,
        "clientNonce":nonce,"signature":STANDARD.encode(signature)}),
    )
    .map_err(|_| Error::Json)
}

#[cfg(test)]
#[path = "device_session_proof_tests.rs"]
mod tests;
