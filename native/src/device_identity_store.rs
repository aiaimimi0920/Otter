//! 显式目录中的设备身份存储；合作实例用独占 lock 文件协调，外部非合作写入非 CAS。
use crate::{
    device_identity::{Identity, MAX_DOCUMENT},
    device_identity_file,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Missing,
    Read,
    Lock,
    Changed,
    Write,
    Identity(crate::device_identity::Error),
}
pub struct Store {
    root: PathBuf,
    owner: Arc<()>,
}
pub struct Loaded {
    identity: Identity,
    digest: [u8; 32],
    owner: Arc<()>,
}
impl Loaded {
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
}
impl Store {
    pub fn new(root: &Path) -> Result<Self, Error> {
        if !root.is_absolute() {
            return Err(Error::Path);
        }
        Ok(Self {
            root: root.to_owned(),
            owner: Arc::new(()),
        })
    }
    /// 续签入口：缺失返回错误，legacy 不自动重写；不会创建目录或替代身份。
    pub fn load_existing(&self) -> Result<Loaded, Error> {
        let (root, _lock) = self.lock(false)?;
        let bytes = read(&root.join("device-identity.json"))?.ok_or(Error::Missing)?;
        self.loaded(&bytes)
    }
    /// 仅显式初次授权使用：不存在才生成；损坏、权限错误绝不退化为生成新身份。
    pub fn load_or_create(&self) -> Result<Loaded, Error> {
        let (root, _lock) = self.lock(true)?;
        let path = root.join("device-identity.json");
        match read(&path)? {
            Some(bytes) => {
                let loaded = self.loaded(&bytes)?;
                if !loaded.identity.needs_migration() {
                    return Ok(loaded);
                }
                let protected = loaded
                    .identity
                    .protected_document()
                    .map_err(Error::Identity)?;
                let next = self.loaded(&protected)?;
                self.check_current(&path, &loaded)?;
                device_identity_file::write(&path, &protected, true).map_err(|_| Error::Write)?;
                Ok(next)
            }
            None => {
                let bytes = Identity::generate()
                    .map_err(Error::Identity)?
                    .protected_document()
                    .map_err(Error::Identity)?;
                let next = self.loaded(&bytes)?;
                device_identity_file::write(&path, &bytes, false).map_err(|_| Error::Write)?;
                Ok(next)
            }
        }
    }
    /// 外层确认配对响应后调用。失败保留旧 Loaded；检测到过期快照拒绝覆盖。
    pub fn bind_and_save(&self, loaded: &mut Loaded, device: &str) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.owner, &loaded.owner) {
            return Err(Error::Changed);
        }
        let (root, _lock) = self.lock(false)?;
        let path = root.join("device-identity.json");
        self.check_current(&path, loaded)?;
        let mut candidate = Identity::parse(
            &loaded
                .identity
                .protected_document()
                .map_err(Error::Identity)?,
        )
        .map_err(Error::Identity)?;
        candidate.bind(device).map_err(Error::Identity)?;
        let bytes = candidate.protected_document().map_err(Error::Identity)?;
        let next = self.loaded(&bytes)?;
        self.check_current(&path, loaded)?;
        device_identity_file::write(&path, &bytes, true).map_err(|_| Error::Write)?;
        *loaded = next;
        Ok(())
    }
    fn loaded(&self, bytes: &[u8]) -> Result<Loaded, Error> {
        Ok(Loaded {
            identity: Identity::parse(bytes).map_err(Error::Identity)?,
            digest: Sha256::digest(bytes).into(),
            owner: self.owner.clone(),
        })
    }
    fn check_current(&self, path: &Path, loaded: &Loaded) -> Result<(), Error> {
        let bytes = read(path)?.ok_or(Error::Changed)?;
        if <[u8; 32]>::from(Sha256::digest(&bytes)) != loaded.digest {
            return Err(Error::Changed);
        }
        Ok(())
    }
    fn lock(&self, create: bool) -> Result<(PathBuf, File), Error> {
        if create {
            std::fs::create_dir_all(&self.root).map_err(|_| Error::Path)?;
        }
        let metadata = std::fs::symlink_metadata(&self.root).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::Missing
            } else {
                Error::Path
            }
        })?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(Error::Path);
        }
        let root = self.root.canonicalize().map_err(|_| Error::Path)?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join(".device-identity.lock"))
            .map_err(|_| Error::Lock)?;
        regular(&lock).map_err(|_| Error::Lock)?;
        Ok((root, lock))
    }
}

fn regular(file: &File) -> Result<(), Error> {
    let metadata = file.metadata().map_err(|_| Error::Read)?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::Read);
    }
    Ok(())
}
fn read(path: &Path) -> Result<Option<Zeroizing<Vec<u8>>>, Error> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Read),
    };
    regular(&file)?;
    if file.metadata().map_err(|_| Error::Read)?.len() > MAX_DOCUMENT as u64 {
        return Err(Error::Identity(crate::device_identity::Error::Limit));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take((MAX_DOCUMENT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Read)?;
    if bytes.len() > MAX_DOCUMENT {
        return Err(Error::Identity(crate::device_identity::Error::Limit));
    }
    Ok(Some(bytes))
}

#[cfg(test)]
#[path = "device_identity_store_tests.rs"]
mod tests;
