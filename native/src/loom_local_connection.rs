//! 本机 Loom 的生产投射入口：清单授权只做本机登记，LAN 始终使用签名 Device 会话。
use crate::{
    device_identity_store::Store, device_request_cancel::Cancel, device_session_attempt::Session,
    device_session_client::Client, loom_discovery,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rustls::{pki_types::CertificateDer, RootCertStore};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
#[path = "loom_local_connection_http.rs"]
mod http;
#[path = "loom_local_connection_owner.rs"]
pub(crate) mod owner;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Discovery,
    LocalOnly,
    LocalHttp,
    LanDisabled,
    Response,
    DeviceDenied,
    AwaitingApproval,
    Store,
    Tls,
    DisconnectConfig(crate::device_authorization_config::Error),
    DisconnectExchange(crate::device_session_client::Error),
    Stopped,
    Worker,
}
pub(crate) struct Connected {
    pub origin: String,
    pub session: Session,
    pub roots: RootCertStore,
    pub profile: PathBuf,
    pub name: String,
    pub loom_id: String,
}
type TrustEntry = (PathBuf, String, RootCertStore);
static TRUST: OnceLock<Mutex<Vec<TrustEntry>>> = OnceLock::new();
pub(crate) fn roots(profile: &Path, origin: &str) -> Option<RootCertStore> {
    TRUST
        .get()?
        .lock()
        .ok()?
        .iter()
        .find(|(p, o, _)| p == profile && o == origin)
        .map(|(_, _, r)| r.clone())
}
pub(crate) fn remember(connected: &Connected) -> Result<(), Error> {
    let mut entries = TRUST
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map_err(|_| Error::Tls)?;
    if let Some(entry) = entries.iter_mut().find(|(p, _, _)| p == &connected.profile) {
        entry.1 = connected.origin.clone();
        entry.2 = connected.roots.clone();
    } else {
        if entries.len() >= 8 {
            return Err(Error::Tls);
        }
        entries.push((
            connected.profile.clone(),
            connected.origin.clone(),
            connected.roots.clone(),
        ));
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    id: String,
    public_key: Option<String>,
    approval: String,
    enabled: bool,
}
#[derive(Deserialize)]
struct Registry {
    devices: Vec<Device>,
    pending: Vec<Device>,
}
fn own_device(value: Value, public: &str) -> Result<Option<Device>, Error> {
    let registry: Registry = serde_json::from_value(value).map_err(|_| Error::Response)?;
    let mut devices = registry
        .devices
        .into_iter()
        .chain(registry.pending)
        .filter(|d| d.public_key.as_deref() == Some(public));
    let Some(device) = devices.next() else {
        return Ok(None);
    };
    if devices.next().is_some()
        || !device.enabled
        || !matches!(device.approval.as_str(), "pending" | "approved")
        || device.id.is_empty()
        || device.id.len() > 160
        || !device
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err(Error::DeviceDenied);
    }
    Ok(Some(device))
}
pub(crate) fn connect(
    discovery: loom_discovery::Config,
    profile: &Path,
    cancel: &Cancel,
) -> Result<Connected, Error> {
    if let Some(remote) = crate::loom_remote_connection::Document::load(profile)? {
        return remote.connect(profile, cancel);
    }
    connect_local(discovery, profile, cancel)
}
fn connect_local(
    discovery: loom_discovery::Config,
    profile: &Path,
    cancel: &Cancel,
) -> Result<Connected, Error> {
    let local = http::Local::new(discovery.load_media().map_err(|_| Error::Discovery)?)?;
    let info = local.request("/v1/otter/identity", None, cancel)?;
    connect_discovered_local(local, info, profile, cancel)
}
fn connect_discovered_local(
    local: http::Local,
    info: Value,
    profile: &Path,
    cancel: &Cancel,
) -> Result<Connected, Error> {
    if info["protocolVersion"] != "loom.connection.v1" || info["discoverable"] != true {
        return Err(Error::LanDisabled);
    }
    let loom_id = info["loomId"]
        .as_str()
        .filter(|id| crate::projection_delivery::share::loom_id(id))
        .ok_or(Error::Response)?;
    let origin = info["endpoint"].as_str().ok_or(Error::Response)?;
    let origin =
        crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Response)?;
    let certificate = info["certificate"]
        .as_str()
        .filter(|s| s.len() <= 8192)
        .ok_or(Error::Response)?;
    let certificate = STANDARD.decode(certificate).map_err(|_| Error::Tls)?;
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(certificate))
        .map_err(|_| Error::Tls)?;
    let url = url::Url::parse(&origin).map_err(|_| Error::Response)?;
    let host = url
        .host_str()
        .ok_or(Error::Response)?
        .trim_matches(['[', ']']);
    let addresses = crate::loom_address_resolver::resolve(
        host,
        url.port_or_known_default().ok_or(Error::Response)?,
        cancel,
    )
    .map_err(|_| Error::Tls)?;
    let client = Client::new(&origin, roots.clone(), addresses).map_err(|_| Error::Tls)?;
    // 在登记前完成真实 TLS 身份读取；不把清单或广播声明当作握手成功。
    let (status, bytes) = client
        .exchange(
            crate::device_session_client::Request {
                method: "GET",
                path: "/v1/otter/identity",
                body: &[],
                device: None,
                limit: 16 * 1024,
            },
            cancel.flag(),
            &mut |handle| cancel.install(handle),
            std::time::Instant::now() + std::time::Duration::from_secs(10),
        )
        .map_err(|_| Error::Tls)?;
    cancel.clear();
    let remote: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Response)?;
    if status != 200
        || remote["protocolVersion"] != "loom.connection.v1"
        || remote["loomId"] != loom_id
        || remote["endpoint"] != origin
    {
        return Err(Error::Tls);
    }
    let store = Store::new(profile).map_err(|_| Error::Store)?;
    let mut loaded = store.load_or_create().map_err(|_| Error::Store)?;
    let public = loaded.identity().public_key().map_err(|_| Error::Store)?;
    let registry = local.request("/v1/devices", None, cancel)?;
    let device = match own_device(registry, &public)? {
        Some(device) if device.approval == "approved" => device,
        Some(device) => {
            let registry = local.request(
                &format!("/v1/devices/{}/approve", device.id),
                Some(json!({})),
                cancel,
            )?;
            own_device(registry, &public)?.ok_or(Error::Response)?
        }
        None => {
            let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Otter".into());
            let registry = local.request("/v1/devices", Some(json!({"name":name,"kind":"computer","address":"otter://local","publicKey":public})), cancel)?;
            own_device(registry, &public)?.ok_or(Error::Response)?
        }
    };
    if device.approval != "approved" {
        return Err(Error::DeviceDenied);
    }
    if loaded.identity().device_id() != Some(device.id.as_str()) {
        store
            .bind_and_save(&mut loaded, &device.id)
            .map_err(|_| Error::Store)?;
    }
    let session = client
        .issue_once(loaded.identity(), cancel.flag(), |handle| {
            cancel.install(handle)
        })
        .map_err(|_| Error::Tls)?;
    cancel.clear();
    if cancel.is_stopped() {
        return Err(Error::Stopped);
    }
    let name = crate::live_relay_http::Control::new(&client, &session, cancel)
        .loom_account(loom_id)
        .map_err(|_| Error::Response)?;
    Ok(Connected {
        origin,
        session,
        roots,
        profile: profile.into(),
        name: name.unwrap_or_else(|| "未登录账号".into()),
        loom_id: loom_id.into(),
    })
}
pub(crate) fn connect_selected_local(
    origin: &str,
    profile: &Path,
    cancel: &Cancel,
) -> Option<Result<Connected, Error>> {
    let local = http::Local::new(
        crate::loom_discovery::Config::from_env()
            .load_media()
            .ok()?,
    )
    .ok()?;
    let info = local.request("/v1/otter/identity", None, cancel).ok()?;
    (info["endpoint"] == origin).then(|| connect_discovered_local(local, info, profile, cancel))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_local_connection_tests.rs"]
mod tests;
