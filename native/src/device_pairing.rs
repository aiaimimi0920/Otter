//! 显式配对注册；只发送公钥并保存关联，绝不把注册响应当作批准或会话凭据。
use crate::{
    device_identity::{valid_field, Identity},
    device_identity_store::{Loaded, Store},
    device_session_client::Client,
    loom_bridge_stream::Interrupt,
};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};

const PATH: &str = "/v1/devices/requests";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Name,
    Key,
    Limit,
    Http(u16),
    Json,
    Missing,
    Ambiguous,
    Device,
    Stopped,
    Network(crate::device_session_client::Error),
    Store(crate::device_identity_store::Error),
}
struct Request {
    public: String,
    body: Vec<u8>,
}
impl Request {
    fn new(identity: &Identity, name: &str) -> Result<Self, Error> {
        let name = name.trim();
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            return Err(Error::Name);
        }
        let public = identity.public_key().map_err(|_| Error::Key)?;
        let body = serde_json::to_vec(&serde_json::json!({
            "name": name, "kind": "computer", "address": format!("hook://{name}"),
            "publicKey": public,
        }))
        .map_err(|_| Error::Json)?;
        Ok(Self { public, body })
    }
    fn finish(self, status: u16, body: &[u8]) -> Result<String, Error> {
        if body.len() > crate::device_session_attempt::MAX_RESPONSE {
            return Err(Error::Limit);
        }
        if !(200..300).contains(&status) {
            return Err(Error::Http(status));
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Device {
            id: String,
            #[serde(default)]
            public_key: Option<String>,
        }
        #[derive(Deserialize)]
        struct Registry {
            #[serde(default)]
            devices: Vec<Device>,
            #[serde(default)]
            pending: Vec<Device>,
        }
        let registry: Registry = serde_json::from_slice(body).map_err(|_| Error::Json)?;
        let mut selected = None;
        for device in registry.pending.into_iter().chain(registry.devices) {
            if device.public_key.as_deref() != Some(self.public.as_str()) {
                continue;
            }
            if !valid_field(&device.id, 160) {
                return Err(Error::Device);
            }
            // 同一公钥出现不同 ID 时不猜测关联目标；相同 ID 的重复记录可接受。
            if selected.as_ref().is_some_and(|id| id != &device.id) {
                return Err(Error::Ambiguous);
            }
            selected = Some(device.id);
        }
        selected.ok_or(Error::Missing)
    }
}

/// 仅取得该 endpoint 返回的公钥关联，不修改默认身份；适用于连接专属关联。
pub fn register(
    client: &Client,
    identity: &Identity,
    name: &str,
    stop: &AtomicBool,
    mut connected: impl FnMut(Interrupt),
) -> Result<String, Error> {
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    let request = Request::new(identity, name)?;
    let (status, body) = client
        .post(PATH, &request.body, stop, &mut connected)
        .map_err(Error::Network)?;
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    request.finish(status, &body)
}

/// 显式默认关联配对。调用者先通过 Store::load_or_create 取得身份，不在此处生成密钥。
/// 成功后上层必须撤销此 origin 的旧授权；返回值仅表示设备 ID 是否改变，不表示批准。
pub fn register_and_save(
    client: &Client,
    store: &Store,
    loaded: &mut Loaded,
    name: &str,
    stop: &AtomicBool,
    connected: impl FnMut(Interrupt),
) -> Result<bool, Error> {
    let device = register(client, loaded.identity(), name, stop, connected)?;
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    let changed = loaded.identity().device_id() != Some(device.as_str());
    // 提交检查点后以文件事务完成，不在落盘中途取消；失败不会改写调用方 Loaded。
    // 网络注册可能已成功但本地保存失败，必须报告 Store 错误，不能返回配对完成。
    store.bind_and_save(loaded, &device).map_err(Error::Store)?;
    Ok(changed)
}

#[cfg(test)]
#[path = "device_pairing_tests.rs"]
mod tests;
