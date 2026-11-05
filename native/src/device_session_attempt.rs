//! 单次设备会话协议；状态消费阻止重复使用 challenge，网络与批准轮询由外层负责。
use crate::{device_identity::Identity, device_session_proof};
use serde::{Deserialize, Deserializer};
use url::Url;
use zeroize::Zeroizing;

pub const MAX_RESPONSE: usize = 256 * 1024;
pub const CHALLENGE_PATH: &str = "/v1/device-sessions/challenges";
pub const SESSION_PATH: &str = "/v1/device-sessions";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Origin,
    Identity,
    Limit,
    PendingApproval,
    Http(u16),
    Json,
    Proof(device_session_proof::Error),
    Credential,
    Expired,
}

pub struct Attempt<'a> {
    identity: &'a Identity,
    origin: String,
}
pub struct SignedAttempt {
    device: String,
    origin: String,
    proof: Vec<u8>,
}
// 不实现 Debug/Clone/Serialize；凭据只允许发往申请时固定的 HTTPS origin。
pub struct Session {
    device: String,
    origin: String,
    token: Zeroizing<String>,
    expires_at_ms: u64,
}

impl<'a> Attempt<'a> {
    pub fn new(identity: &'a Identity, origin: &str) -> Result<Self, Error> {
        identity.device_id().ok_or(Error::Identity)?;
        Ok(Self {
            identity,
            origin: canonical_origin(origin)?,
        })
    }
    pub fn challenge_body(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"deviceId": self.identity.device_id()}))
            .expect("serializing a device identifier cannot fail")
    }
    /// body 必须来自此 origin 的无重定向 HTTPS 请求；传输层仍须限制读取分配。
    pub fn challenge(self, status: u16, body: &[u8]) -> Result<SignedAttempt, Error> {
        check_size(body)?;
        if !(200..300).contains(&status) {
            #[derive(Deserialize)]
            struct Failure {
                error: Code,
            }
            #[derive(Deserialize)]
            struct Code {
                code: String,
            }
            if status == 403
                && serde_json::from_slice::<Failure>(body)
                    .is_ok_and(|failure| failure.error.code == "device_not_authorized")
            {
                return Err(Error::PendingApproval);
            }
            return Err(Error::Http(status));
        }
        let proof = device_session_proof::create(self.identity, body).map_err(Error::Proof)?;
        Ok(SignedAttempt {
            device: self.identity.device_id().ok_or(Error::Identity)?.to_owned(),
            origin: self.origin,
            proof,
        })
    }
}
impl SignedAttempt {
    pub fn body(&self) -> &[u8] {
        &self.proof
    }
    /// 不重试、不重新配对；调用者提供响应接收时刻，而非请求发出时刻。
    pub fn finish(self, status: u16, body: &[u8], now_ms: u64) -> Result<Session, Error> {
        check_size(body)?;
        if !(200..300).contains(&status) {
            return Err(Error::Http(status));
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            device_id: String,
            #[serde(deserialize_with = "secret")]
            token: Zeroizing<String>,
            expires_at_ms: u64,
        }
        let response: Response = serde_json::from_slice(body).map_err(|_| Error::Json)?;
        if response.device_id != self.device {
            return Err(Error::Identity);
        }
        if !crate::device_identity::valid_field(&response.token, 8192) {
            return Err(Error::Credential);
        }
        if response.expires_at_ms <= now_ms {
            return Err(Error::Expired);
        }
        Ok(Session {
            device: self.device,
            origin: self.origin,
            token: response.token,
            expires_at_ms: response.expires_at_ms,
        })
    }
}
impl Session {
    pub fn device_id(&self) -> &str {
        &self.device
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
    pub fn token_for(&self, origin: &str, now_ms: u64) -> Result<&str, Error> {
        if canonical_origin(origin)? != self.origin {
            return Err(Error::Origin);
        }
        if self.expires_at_ms <= now_ms {
            return Err(Error::Expired);
        }
        Ok(&self.token)
    }
}
fn secret<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}
fn check_size(body: &[u8]) -> Result<(), Error> {
    if body.len() > MAX_RESPONSE {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
pub(super) fn canonical_origin(value: &str) -> Result<String, Error> {
    if value.len() > 2048 || value.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(Error::Origin);
    }
    let url = Url::parse(value).map_err(|_| Error::Origin)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Origin);
    }
    Ok(url.origin().ascii_serialization())
}

#[cfg(test)]
#[path = "device_session_attempt_tests.rs"]
mod tests;
