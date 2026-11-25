//! Immutable, origin-isolated operator-CA snapshots; do not modify the OS trust store or trust certificates supplied by the manifest.
use rustls::{
    pki_types::{pem::PemObject, CertificateDer},
    RootCertStore,
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read},
    path::Path,
};

const MAX_BYTES: usize = 32 * 1024;
const MAX_CERTIFICATES: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Origin,
    Path,
    Read,
    Limit,
    Pem,
    Certificate,
}

pub struct Scoped {
    origin: String,
    digest: [u8; 32],
    roots: RootCertStore,
}
impl Scoped {
    pub fn from_pem(origin: &str, pem: &[u8]) -> Result<Self, Error> {
        let origin = https_origin(origin)?;
        if pem.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let mut remaining = std::str::from_utf8(pem).map_err(|_| Error::Pem)?;
        let mut roots = RootCertStore::empty();
        let mut count = 0;
        while !remaining.trim().is_empty() {
            if count == MAX_CERTIFICATES {
                return Err(Error::Limit);
            }
            remaining = remaining.trim_start();
            if !remaining.starts_with("-----BEGIN CERTIFICATE-----") {
                return Err(Error::Pem);
            }
            let end = remaining
                .find("-----END CERTIFICATE-----")
                .ok_or(Error::Pem)?
                + "-----END CERTIFICATE-----".len();
            let certificate = CertificateDer::from_pem_slice(&remaining.as_bytes()[..end])
                .map_err(|_| Error::Pem)?;
            roots.add(certificate).map_err(|_| Error::Certificate)?;
            count += 1;
            remaining = &remaining[end..];
        }
        if count == 0 {
            return Err(Error::Pem);
        }
        Ok(Self {
            origin,
            digest: Sha256::digest(pem).into(),
            roots,
        })
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.digest
    }
}

/// Default roots come from the platform/release-configuration owner; this module only adds operator CAs and does not fabricate a public-root list.
pub struct Trust {
    defaults: RootCertStore,
    scoped: Option<Scoped>,
}
impl Trust {
    /// Compile-time Mozilla public roots, not the Windows system certificate store.
    pub fn public_roots() -> RootCertStore {
        RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        }
    }

    /// The background configuration owner initializes this snapshot once and retains it; do not call per frame/service invocation.
    pub fn load_default() -> Result<Self, Error> {
        Self::load(Self::public_roots())
    }

    pub fn new(defaults: RootCertStore, scoped: Option<Scoped>) -> Self {
        Self { defaults, scoped }
    }

    /// Initialize once in the background; retain the snapshot until explicitly recreated, without polling files on frame/network hot paths.
    pub fn load(defaults: RootCertStore) -> Result<Self, Error> {
        Self::load_with(
            defaults,
            |name| std::env::var_os(name),
            |path| {
                let file = File::open(path)?;
                if !file.metadata()?.is_file() {
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                Ok(file)
            },
        )
    }

    /// The canonical OTTER variable takes precedence when present, including empty values; old HOOK values cannot bypass invalid configuration.
    pub fn load_with<R: Read>(
        defaults: RootCertStore,
        lookup: impl Fn(&str) -> Option<OsString>,
        mut open: impl FnMut(&Path) -> io::Result<R>,
    ) -> Result<Self, Error> {
        let read = |suffix: &str| {
            lookup(&format!("OTTER_LOOM_TLS_{suffix}"))
                .or_else(|| lookup(&format!("HOOK_LOOM_TLS_{suffix}")))
        };
        let (origin, path) = match (read("ORIGIN"), read("CA_FILE")) {
            (None, None) => return Ok(Self::new(defaults, None)),
            (Some(origin), Some(path)) => (origin, path),
            _ => return Err(Error::Configuration),
        };
        let origin = origin.to_str().ok_or(Error::Origin)?;
        https_origin(origin)?;
        let path = Path::new(&path);
        if !path.is_absolute() {
            return Err(Error::Path);
        }
        let reader = open(path).map_err(|_| Error::Read)?;
        let mut bytes = Vec::new();
        reader
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Read)?;
        Ok(Self::new(defaults, Some(Scoped::from_pem(origin, &bytes)?)))
    }

    /// Only the exact HTTPS origin, with canonical host/default port, receives the additional CA; HTTP does not inherit it.
    pub fn roots_for(&self, origin: &str) -> Result<RootCertStore, Error> {
        let url = origin_url(origin)?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::Origin);
        }
        let mut roots = self.defaults.clone();
        if let Some(scoped) = &self.scoped {
            if url.scheme() == "https" && url.origin().ascii_serialization() == scoped.origin {
                roots.extend(scoped.roots.roots.iter().cloned());
            }
        }
        Ok(roots)
    }
}

fn origin_url(value: &str) -> Result<url::Url, Error> {
    if value.len() > 2048 || value.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(Error::Origin);
    }
    let url = url::Url::parse(value).map_err(|_| Error::Origin)?;
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Origin);
    }
    Ok(url)
}
fn https_origin(value: &str) -> Result<String, Error> {
    let url = origin_url(value)?;
    if url.scheme() != "https" {
        return Err(Error::Origin);
    }
    Ok(url.origin().ascii_serialization())
}

#[cfg(test)]
#[path = "loom_tls_trust_tests.rs"]
mod tests;
