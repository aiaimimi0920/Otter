//! 操作员 CA 的不可变、origin 隔离快照；不修改 OS 信任库，不信任清单自带证书。
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

/// 默认 roots 由平台/发布配置 owner 提供；此模块只附加操作员 CA，不伪造公共根列表。
pub struct Trust {
    defaults: RootCertStore,
    scoped: Option<Scoped>,
}
impl Trust {
    /// 编译期 Mozilla 公共根；不是 Windows 系统证书库。
    pub fn public_roots() -> RootCertStore {
        RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        }
    }

    /// 后台配置 owner 初始化一次后保留本快照；不在每帧/每次 service 中调用。
    pub fn load_default() -> Result<Self, Error> {
        Self::load(Self::public_roots())
    }

    pub fn new(defaults: RootCertStore, scoped: Option<Scoped>) -> Self {
        Self { defaults, scoped }
    }

    /// 在后台初始化一次；持有快照直至显式重新创建，不在帧/网络热路径轮询文件。
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

    /// 规范 OTTER 变量存在（含空值）时优先，不能用旧 HOOK 值绕过错误配置。
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

    /// 仅精确 HTTPS origin（规范 host/默认端口）获得附加 CA；HTTP 不继承它。
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
