//! 显式导入的远端 Loom 配置；只保存公开地址和证书，身份与会话沿现有设备协议。
use crate::{
    device_identity_store::Store,
    device_request_cancel::Cancel,
    device_session_client::Client,
    loom_local_connection::{Connected, Error},
    loom_tls_trust::{Scoped, Trust},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Document {
    pub schema_version: u32,
    pub origin: String,
    pub loom_id: String,
    pub certificate_pem: String,
    #[serde(default)]
    pub device_id: Option<String>,
}
impl Document {
    pub(crate) fn load(profile: &Path) -> Result<Option<Self>, Error> {
        let Some(bytes) = Self::read(profile)? else {
            return Ok(None);
        };
        let value: Self = serde_json::from_slice(&bytes).map_err(|_| Error::Response)?;
        if value.schema_version != 1
            || value.loom_id.is_empty()
            || value.loom_id.len() > 256
            || crate::device_session_attempt::canonical_origin(&value.origin).as_ref()
                != Ok(&value.origin)
            || value
                .device_id
                .as_ref()
                .is_some_and(|id| !crate::device_identity::valid_field(id, 160))
        {
            return Err(Error::Response);
        }
        value.roots()?;
        Ok(Some(value))
    }
    pub(crate) fn read(profile: &Path) -> Result<Option<Vec<u8>>, Error> {
        crate::profile_directory::validate(profile).map_err(|_| Error::Store)?;
        let path = profile.join("loom-remote.json");
        let file = match OpenOptions::new()
            .read(true)
            .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
            .custom_flags(windows::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Store),
        };
        let metadata = file.metadata().map_err(|_| Error::Store)?;
        if !metadata.is_file()
            || metadata.file_attributes()
                & windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT.0
                != 0
        {
            return Err(Error::Store);
        }
        let mut bytes = Vec::new();
        file.take(40 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Store)?;
        if bytes.len() > 40 * 1024 {
            return Err(Error::Response);
        }
        Ok(Some(bytes))
    }
    fn roots(&self) -> Result<rustls::RootCertStore, Error> {
        if self.certificate_pem.is_empty() {
            return Ok(Trust::public_roots());
        }
        let scoped = Scoped::from_pem(&self.origin, self.certificate_pem.as_bytes())
            .map_err(|_| Error::Tls)?;
        Trust::new(Trust::public_roots(), Some(scoped))
            .roots_for(&self.origin)
            .map_err(|_| Error::Tls)
    }
    pub(crate) fn verify(
        &self,
        cancel: &Cancel,
    ) -> Result<(Client, rustls::RootCertStore, serde_json::Value), Error> {
        let roots = self.roots()?;
        let url = url::Url::parse(&self.origin).map_err(|_| Error::Response)?;
        let addresses = crate::loom_address_resolver::resolve(
            url.host_str()
                .ok_or(Error::Response)?
                .trim_matches(['[', ']']),
            url.port_or_known_default().ok_or(Error::Response)?,
            cancel,
        )
        .map_err(|_| Error::Tls)?;
        let client = Client::new(&self.origin, roots.clone(), addresses).map_err(|_| Error::Tls)?;
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
        let info: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| Error::Response)?;
        if status != 200
            || info["protocolVersion"] != "loom.connection.v1"
            || info["loomId"] != self.loom_id
        {
            return Err(Error::Tls);
        }
        Ok((client, roots, info))
    }
    pub(crate) fn connect(self, profile: &Path, cancel: &Cancel) -> Result<Connected, Error> {
        self.connect_with_password(profile, cancel, "")
    }
    pub(crate) fn connect_with_password(
        mut self,
        profile: &Path,
        cancel: &Cancel,
        password: &str,
    ) -> Result<Connected, Error> {
        let (client, roots, _) = self.verify(cancel)?;
        let store = Store::new(profile).map_err(|_| Error::Store)?;
        let mut loaded = store.load_or_create().map_err(|_| Error::Store)?;
        // 只在首次明确导入连接且尚无设备关联时注册；撤权或续签失败绝不重新配对。
        if self.device_id.is_none() {
            let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Otter".into());
            let body = zeroize::Zeroizing::new(serde_json::to_vec(&serde_json::json!({"name":name,"publicKey":loaded.identity().public_key().map_err(|_| Error::Store)?,"password":password})).map_err(|_| Error::Response)?);
            let (status, bytes) = client
                .post(
                    "/v1/otter/connections/request",
                    &body,
                    cancel.flag(),
                    &mut |handle| cancel.install(handle),
                )
                .map_err(|_| Error::Tls)?;
            cancel.clear();
            if status != 201 {
                return Err(Error::DeviceDenied);
            }
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| Error::Response)?;
            let id = value["connection"]["deviceId"]
                .as_str()
                .filter(|id| crate::device_identity::valid_field(id, 160))
                .ok_or(Error::Response)?;
            self.device_id = Some(id.into());
            crate::loom_selected_connection::save(profile, &self, cancel)?;
        }
        if loaded.identity().device_id() != self.device_id.as_deref() {
            store
                .bind_and_save(&mut loaded, self.device_id.as_deref().ok_or(Error::Store)?)
                .map_err(|_| Error::Store)?;
        }
        let session = client
            .issue_once(loaded.identity(), cancel.flag(), |handle| {
                cancel.install(handle)
            })
            .map_err(|error| match error {
                // PendingApproval is also used for revoked devices: never auto-register again.
                // Polling only retries signed session requests; it cannot restore administrator approval.
                crate::device_session_client::Error::Protocol(
                    crate::device_session_attempt::Error::PendingApproval,
                ) => Error::AwaitingApproval,
                crate::device_session_client::Error::Protocol(
                    crate::device_session_attempt::Error::Http(401 | 403 | 404),
                ) => Error::DeviceDenied,
                _ => Error::Tls,
            })?;
        cancel.clear();
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        let (loom_id, name) = crate::live_relay_http::Control::new(&client, &session, cancel)
            .loom_connection_info()
            .map_err(|_| Error::Response)?;
        if loom_id != self.loom_id {
            return Err(Error::Response);
        }
        Ok(Connected {
            origin: self.origin,
            session,
            roots,
            profile: profile.into(),
            name: name.unwrap_or_else(|| "未登录账号".into()),
            loom_id,
        })
    }
}
