//! 主干兼容的本机发现策略；环境和文件视为本地配置，不声称已验证文件 ACL/所有者。
use crate::{
    loom_bridge_client::{self, BridgeSocket},
    loom_bridge_identity::Identity,
    loom_discovery_manifest,
};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Config {
    enabled: bool,
    paths: Vec<PathBuf>,
    endpoint_override: Option<String>,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Disabled,
    NotFound,
    Read,
    Manifest(loom_discovery_manifest::Error),
    Endpoint,
    Connect(loom_bridge_client::Error),
}

impl Config {
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var_os(name))
    }
    /// 注入只读查询器，不修改全局环境；规范变量即使为空，也优先于旧 HOOK 名称。
    pub fn from_lookup(read: impl Fn(&OsStr) -> Option<OsString>) -> Self {
        let lookup = |name: &str| {
            let canonical = (name.starts_with("HOOK_") || name.starts_with("LOOM_HOOK_"))
                .then(|| name.replace("HOOK", "OTTER"));
            canonical
                .and_then(|name| read(OsStr::new(&name)))
                .or_else(|| read(OsStr::new(name)))
        };
        let enabled = lookup("HOOK_ENABLE_LOOM_HOOK")
            .and_then(|value| value.into_string().ok())
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(true);
        let mut paths = Vec::new();
        let path = |name| {
            lookup(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        if let Some(value) = path("LOOM_MANIFEST_PATH") {
            paths.push(value);
        }
        for key in [
            "LOOM_MANIFEST_DIR",
            "LOOM_CAPABILITY_MANIFEST_DIR",
            "NEURO_CAPABILITIES_DIR",
        ] {
            if let Some(value) = path(key) {
                paths.push(value.join("loom.json"));
            }
        }
        if let Some(value) = path("APPDATA") {
            paths.push(value.join("Neuro").join("capabilities").join("loom.json"));
        }
        paths.push(
            PathBuf::from(".runtime")
                .join("neuro")
                .join("capabilities")
                .join("loom.json"),
        );
        Self {
            enabled,
            paths,
            endpoint_override: lookup("LOOM_HOOK_WS_URL")
                .and_then(|value| value.into_string().ok()),
        }
    }
    pub fn load(&self) -> Result<Identity, Error> {
        self.load_with(|path| File::open(path))
    }
    /// 只在 NotFound 时继续候选；读取/解析失败必须停止，不能静默使用较低优先级身份。
    pub fn load_with<R: Read>(
        &self,
        open: impl FnMut(&Path) -> io::Result<R>,
    ) -> Result<Identity, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let identity = self.load_parsed(open, loom_discovery_manifest::parse)?;
        identity
            .check_endpoint_override(self.endpoint_override.as_deref())
            .map_err(|_| Error::Endpoint)?;
        Ok(identity)
    }

    /// HTTP 媒体发现不受 hook bridge 的启停/WS override 控制。
    pub fn load_media(&self) -> Result<crate::loom_media_target::Target, Error> {
        self.load_media_with(|path| File::open(path))
    }
    pub fn load_media_with<R: Read>(
        &self,
        open: impl FnMut(&Path) -> io::Result<R>,
    ) -> Result<crate::loom_media_target::Target, Error> {
        self.load_parsed(open, loom_discovery_manifest::parse_media)
    }
    fn load_parsed<R: Read, T>(
        &self,
        mut open: impl FnMut(&Path) -> io::Result<R>,
        parse: impl Fn(&[u8]) -> Result<T, loom_discovery_manifest::Error>,
    ) -> Result<T, Error> {
        for path in &self.paths {
            let reader = match open(path) {
                Ok(reader) => reader,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(_) => return Err(Error::Read),
            };
            let mut bytes = Vec::new();
            reader
                .take((loom_discovery_manifest::MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::Read)?;
            return parse(&bytes).map_err(Error::Manifest);
        }
        Err(Error::NotFound)
    }
}

/// 必须由后台 owner 调用；默认开启行为与主干一致，失败错误不包含清单路径或凭据。
pub fn connect_default(timeout: Duration) -> Result<BridgeSocket, Error> {
    let identity = Config::from_env().load()?;
    loom_bridge_client::connect(&identity, timeout).map_err(Error::Connect)
}

#[cfg(test)]
#[path = "loom_discovery_tests.rs"]
mod tests;
