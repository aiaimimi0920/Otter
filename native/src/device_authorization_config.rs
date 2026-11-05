//! 授权配置快照：前台仅校验/捕获配置，文件信任加载与原生解析在授权线程执行。
use crate::{
    device_authorization_runtime::Task, device_identity_store::Store,
    device_request_cancel::Cancel, device_session_client::Client, loom_tls_trust::Trust,
};
use std::{
    ffi::OsString,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    RequestPairing,
    AwaitApproval,
    Renew,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Origin,
    Profile,
    Name,
    Stopped,
    Trust,
    Resolve(crate::loom_address_resolver::Error),
    Client(crate::device_session_client::Error),
    Store(crate::device_identity_store::Error),
    Credential,
    Clock,
}

pub struct Config {
    origin: String,
    root: PathBuf,
    name: String,
    trust_env: Vec<(String, OsString)>,
}
impl Config {
    /// origin 必须由连接选择明确传入；TLS_ORIGIN 只限定 CA，绝不是默认服务地址。
    /// 不读取身份/CA 文件、不创建目录、不做 DNS，适合菜单回调外的请求准备。
    pub fn from_env(origin: &str) -> Result<Self, Error> {
        Self::from_lookup(origin, |key| std::env::var_os(key))
    }
    pub fn from_lookup(
        origin: &str,
        lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self, Error> {
        let override_root = lookup("OTTER_NATIVE_PROFILE_DIR");
        let local = lookup("LOCALAPPDATA");
        let selection = crate::settings_profile::select(
            None,
            override_root.as_deref(),
            local.as_deref(),
            false,
        )
        .map_err(|_| Error::Profile)?
        .ok_or(Error::Profile)?;
        let root = selection.path.parent().ok_or(Error::Profile)?;
        let name = lookup("COMPUTERNAME").or_else(|| lookup("HOSTNAME"));
        let name = match name.as_ref() {
            Some(value) => value.to_str().ok_or(Error::Name)?.trim(),
            None => "Otter 设备",
        };
        let name = if name.is_empty() {
            "Otter 设备"
        } else {
            name
        };
        let mut config = Self::new(origin, root, name)?;
        for key in [
            "OTTER_LOOM_TLS_ORIGIN",
            "OTTER_LOOM_TLS_CA_FILE",
            "HOOK_LOOM_TLS_ORIGIN",
            "HOOK_LOOM_TLS_CA_FILE",
        ] {
            if let Some(value) = lookup(key) {
                config.trust_env.push((key.into(), value));
            }
        }
        Ok(config)
    }
    /// 显式路径用于选定 profile/测试；不自动继承任何当前用户 TLS 环境配置。
    pub fn new(origin: &str, root: &Path, name: &str) -> Result<Self, Error> {
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        crate::settings_profile::select(None, Some(root.as_os_str()), None, false)
            .map_err(|_| Error::Profile)?;
        let name = name.trim();
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            return Err(Error::Name);
        }
        Ok(Self {
            origin,
            root: root.to_owned(),
            name: name.into(),
            trust_env: Vec::new(),
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn identity_root(&self) -> &Path {
        &self.root
    }
    /// 显式清除的减权提交不可被随后关闭撤销；无网络、无身份创建，只有有界原子磁盘提交。
    pub(super) fn clear_offline(self) -> Result<(), Error> {
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Clock)?
                .as_millis(),
        )
        .map_err(|_| Error::Clock)?;
        crate::art_offline_store::Slot::invalidate_origin(&self.root, &self.origin, now, &|| false)
            .map_err(|_| Error::Credential)
    }
    pub(super) fn prepare(self, action: Action, cancel: &Cancel) -> Result<Task, Error> {
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Clock)?
                .as_millis(),
        )
        .map_err(|_| Error::Clock)?;
        crate::art_offline_store::Slot::invalidate_origin(&self.root, &self.origin, now, &|| {
            cancel.is_stopped()
        })
        .map_err(|_| Error::Credential)?;
        let client = self.prepare_client(cancel)?;
        let store = Store::new(&self.root).map_err(Error::Store)?;
        Ok(match action {
            Action::RequestPairing => Task::RequestPairing {
                client,
                store,
                name: self.name,
            },
            Action::AwaitApproval => Task::AwaitApproval { client, store },
            Action::Renew => Task::Renew { client, store },
        })
    }
    /// 只在后台执行；Live 使用已签出租约，不需要重新打开或创建身份存储。
    pub(super) fn prepare_client(&self, cancel: &Cancel) -> Result<Client, Error> {
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        let trust = Trust::load_with(
            Trust::public_roots(),
            |key| {
                self.trust_env
                    .iter()
                    .find(|entry| entry.0 == key)
                    .map(|entry| entry.1.clone())
            },
            |path| {
                let file = File::open(path)?;
                if !file.metadata()?.is_file() {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                Ok(file)
            },
        )
        .map_err(|_| Error::Trust)?;
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        let url = url::Url::parse(&self.origin).map_err(|_| Error::Origin)?;
        let host = url
            .host_str()
            .ok_or(Error::Origin)?
            .trim_start_matches('[')
            .trim_end_matches(']');
        let addresses = crate::loom_address_resolver::resolve(
            host,
            url.port_or_known_default().ok_or(Error::Origin)?,
            cancel,
        )
        .map_err(Error::Resolve)?;
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        Client::new(
            &self.origin,
            trust.roots_for(&self.origin).map_err(|_| Error::Trust)?,
            addresses,
        )
        .map_err(Error::Client)
    }
}

#[cfg(test)]
#[path = "device_authorization_config_tests.rs"]
mod tests;
