//! profile内的有界源码缓存，不存grant/token；每次执行仍须新请求的签名许可。
//! 文件名仅由宿主计算的摘要产生。合作进程独占锁，坏缓存当miss，不递归清理用户目录。
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
pub(crate) const MAX_SOURCE: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) struct Store {
    profile: PathBuf,
}
pub(crate) struct Cached {
    pub source: Vec<u8>,
    pub digest: String,
}
pub(crate) fn valid(source: &[u8]) -> bool {
    !source.is_empty()
        && source.len() <= MAX_SOURCE
        && !source.contains(&0)
        && std::str::from_utf8(source).is_ok()
}
impl Store {
    pub(crate) fn new(profile: &Path) -> Self {
        Self {
            profile: profile.to_owned(),
        }
    }
    pub(crate) fn profile(&self) -> &Path {
        &self.profile
    }
    fn lock(&self, create: bool) -> io::Result<(PathBuf, File)> {
        if !self.profile.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        directory(&self.profile)?;
        let root = self.profile.canonicalize()?.join("art-source-cache");
        if create {
            match std::fs::create_dir(&root) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        directory(&root)?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join(".lock"))?;
        regular(&lock)?;
        Ok((root, lock))
    }
    pub(crate) fn load(&self, key: &[u8; 32]) -> io::Result<Option<Cached>> {
        let Some(source) = self.load_named(&name(key))? else {
            return Ok(None);
        };
        if !valid(&source) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Some(Cached {
            digest: format!("{:x}", Sha256::digest(&source)),
            source,
        }))
    }
    pub(crate) fn load_blob(&self, key: &[u8; 32]) -> io::Result<Option<Vec<u8>>> {
        self.load_named(&format!("{:x}.blob", Sha256::digest(key)))
    }
    fn load_named(&self, name: &str) -> io::Result<Option<Vec<u8>>> {
        let (root, _lock) = match self.lock(false) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let file = match OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join(name))
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        regular(&file)?;
        if file.metadata()?.len() > MAX_SOURCE as u64 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut source = Vec::new();
        file.take((MAX_SOURCE + 1) as u64)
            .read_to_end(&mut source)?;
        if source.is_empty() || source.len() > MAX_SOURCE {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Some(source))
    }
    /// 仅在下载验签成功后调用。写失败不破坏在线执行；没有静默淘汰仍有用的缓存。
    pub(crate) fn put(
        &self,
        key: &[u8; 32],
        source: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> io::Result<()> {
        if !valid(source) || stopped() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.put_named(&name(key), source, stopped)
    }
    pub(crate) fn put_blob(
        &self,
        key: &[u8; 32],
        bytes: &[u8],
        stopped: &impl Fn() -> bool,
    ) -> io::Result<()> {
        self.put_named(&format!("{:x}.blob", Sha256::digest(key)), bytes, stopped)
    }
    fn put_named(&self, name: &str, source: &[u8], stopped: &impl Fn() -> bool) -> io::Result<()> {
        if source.is_empty() || source.len() > MAX_SOURCE || stopped() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let (root, _lock) = self.lock(true)?;
        let destination = root.join(name);
        let (mut count, mut bytes) = (0usize, 0u64);
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_name() == ".lock" {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if !metadata.is_file()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            // tmp/未知文件也计费；崩溃遗留不会导致无限缓存增长。
            if entry.path() != destination {
                count += 1;
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or(io::ErrorKind::InvalidData)?;
            }
            if count >= MAX_ENTRIES || bytes > MAX_BYTES {
                return Err(io::ErrorKind::OutOfMemory.into());
            }
        }
        if bytes + source.len() as u64 > MAX_BYTES || stopped() {
            return Err(io::ErrorKind::OutOfMemory.into());
        }
        super::art_source_cache_file::write(&destination, source, stopped)
    }
}
fn name(key: &[u8; 32]) -> String {
    format!(
        "{}.js",
        key.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}
pub(crate) fn directory(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}
pub(crate) fn regular(file: &File) -> io::Result<()> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(())
}
#[cfg(test)]
#[path = "art_source_cache_tests.rs"]
mod tests;
