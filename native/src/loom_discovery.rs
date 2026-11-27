//! Mainline-compatible local discovery policy; environment/files are local configuration, not claimed-verified file ACLs/owners.
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
        #[cfg(windows)]
        if !crate::elevation_status::allows_desktop_integrations() {
            return Self::local_only();
        }
        Self::from_lookup(|name| std::env::var_os(name))
    }
    // Bridge and HTTP media normally have independent enablement. A manual
    // elevated launch must not consult either one's ordinary-user manifests.
    #[cfg(any(windows, test))]
    fn local_only() -> Self {
        Self {
            enabled: false,
            paths: Vec::new(),
            endpoint_override: None,
        }
    }
    /// Inject a read-only query interface without changing global environment; canonical variables outrank old HOOK names even when empty.
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
    /// Try the next candidate only on NotFound; read/parse failure must stop, never silently use lower-priority identity.
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

    /// HTTP media discovery is independent of hook bridge enablement/WS overrides.
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

/// Background-owner calls only; default-enabled behavior matches mainline, and failure errors contain no manifest paths/credentials.
pub fn connect_default(timeout: Duration) -> Result<BridgeSocket, Error> {
    let identity = Config::from_env().load()?;
    loom_bridge_client::connect(&identity, timeout).map_err(Error::Connect)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_discovery_tests.rs"]
mod tests;
