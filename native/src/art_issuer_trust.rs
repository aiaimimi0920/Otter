//! 配置拥有的 Art 签发者信任；不接受网络响应自动注册，撤信任同步撤销已签出的设备租约。
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_http::art_prepare::{Pin, Prepared, Request},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Weak};
#[path = "art_issuer_resume.rs"]
mod resume;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub origin: String,
    pub key_id: String,
    pub public_key: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub issuers: Vec<Entry>,
}
impl Document {
    pub fn validate(&self) -> Result<(), String> {
        if self.issuers.len() > 32 {
            return Err("Too many Art issuers".into());
        }
        let mut origins = std::collections::HashSet::new();
        for entry in &self.issuers {
            let canonical = crate::device_session_attempt::canonical_origin(&entry.origin)
                .map_err(|_| "Invalid Art issuer origin")?;
            if canonical != entry.origin || !origins.insert(&entry.origin) {
                return Err("Noncanonical or duplicate Art issuer origin".into());
            }
            if entry.key_id.is_empty()
                || entry.key_id.len() > 128
                || !entry
                    .key_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            {
                return Err("Invalid Art issuer keyId".into());
            }
            if entry.public_key.len() != 44 {
                return Err("Invalid Art issuer key".into());
            }
            let bytes = STANDARD
                .decode(&entry.public_key)
                .map_err(|_| "Invalid Art issuer key")?;
            if bytes.len() != 32 || STANDARD.encode(&bytes) != entry.public_key {
                return Err("Invalid Art issuer key".into());
            }
        }
        Ok(())
    }
}
/// 一个 profile/应用 owner 持有一个实例；文件加载或显式设置提交成功后才能 replace。
/// 不监视其他进程写盘，不宣称跨进程立即撤权。Drop 也撤销本 owner 签出的请求链。
#[derive(Default)]
pub struct Authority {
    document: Document,
    active: Vec<(String, Weak<Lease>)>,
}
impl Authority {
    pub(crate) fn is_trusted(&self, origin: &str) -> bool {
        self.document.issuers.iter().any(|e| e.origin == origin)
    }
    pub(crate) fn pending_local(&self, origin: &str) -> Result<Lease, Error> {
        if !self.is_trusted(origin) {
            return Err(Error::Trust);
        }
        Ok(Lease::pending_local(origin))
    }
    pub fn new(document: Document) -> Result<Self, String> {
        document.validate()?;
        Ok(Self {
            document,
            active: Vec::new(),
        })
    }
    pub fn from_settings(source: &crate::settings_source::Source) -> Result<Self, String> {
        Self::new(source.document().art_issuer_trust()?)
    }
    pub fn replace(&mut self, document: Document) -> Result<(), String> {
        document.validate()?;
        self.active.retain(|(origin, lease)| {
            let Some(lease) = lease.upgrade() else {
                return false;
            };
            let old = self.document.issuers.iter().find(|e| &e.origin == origin);
            let next = document.issuers.iter().find(|e| &e.origin == origin);
            if old != next {
                lease.revoke();
                return false;
            }
            true
        });
        self.document = document;
        Ok(())
    }
    fn pin(&mut self, origin: &str, lease: &Arc<Lease>) -> Result<Pin, String> {
        self.active.retain(|(_, lease)| lease.strong_count() > 0);
        let entry = self
            .document
            .issuers
            .iter()
            .find(|entry| entry.origin == origin)
            .ok_or("Art issuer is not trusted")?;
        if self.active.len() >= 64 {
            return Err("Too many active Art trust leases".into());
        }
        let key = STANDARD
            .decode(&entry.public_key)
            .map_err(|_| "Invalid Art issuer key")?;
        let pin = Pin {
            key_id: entry.key_id.clone(),
            public_key: key.try_into().map_err(|_| "Invalid Art issuer key")?,
        };
        self.active.push((origin.to_owned(), Arc::downgrade(lease)));
        Ok(pin)
    }
    /// 仅登记租约；先释放 owner 借用，再把 Authorization 移交后台，不持信任锁跨网络。
    pub fn authorize(&mut self, origin: &str, lease: Arc<Lease>) -> Result<Authorization, Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Trust)?
            .as_millis();
        let now = u64::try_from(now).map_err(|_| Error::Trust)?;
        // 待验证local租约只登记信任/取消，不在这里授予执行或网络权；后台必须另验resume。
        if lease.is_local() {
            if lease.execution_stopped() {
                return Err(Error::Trust);
            }
        } else {
            lease.validate(origin, now).map_err(|_| Error::Trust)?;
        }
        let pin = self.pin(origin, &lease).map_err(|_| Error::Trust)?;
        Ok(Authorization {
            origin: origin.to_owned(),
            pin,
            lease,
        })
    }
}
pub struct Authorization {
    origin: String,
    pin: Pin,
    lease: Arc<Lease>,
}
impl Authorization {
    pub(crate) fn prepare_cached(
        self,
        client: &Client,
        request: Request,
        cache: &crate::art_source_cache::Store,
    ) -> Result<Prepared, Error> {
        if client.origin() != self.origin || request.binding.origin != self.origin {
            return Err(Error::Trust);
        }
        Prepared::fetch_resumable(client, self.lease, request, self.pin, cache)
            .map_err(Error::Prepare)
    }
    pub fn prepare(self, client: &Client, request: Request) -> Result<Prepared, Error> {
        if client.origin() != self.origin || request.binding.origin != self.origin {
            return Err(Error::Trust);
        }
        Prepared::fetch(client, self.lease, request, self.pin).map_err(Error::Prepare)
    }
}
impl Drop for Authority {
    fn drop(&mut self) {
        for (_, lease) in &self.active {
            if let Some(lease) = lease.upgrade() {
                lease.revoke();
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Trust,
    Prepare(crate::live_relay_http::art_prepare::Error),
}

#[cfg(test)]
#[path = "art_issuer_trust_tests.rs"]
mod tests;
