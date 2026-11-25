//! Bounded per-profile source cache without grants/tokens; every execution still requires a newly requested signed grant.
//! Filenames derive only from host-computed digests. Cooperating processes use exclusive locks; corrupt caches are misses, without recursive user-directory cleanup.
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
pub const MAX_SOURCE: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_BYTES: u64 = 32 * 1024 * 1024;
pub struct Store {
    profile: PathBuf,
}
pub struct Cached {
    pub source: Vec<u8>,
    pub digest: String,
}
pub fn valid(source: &[u8]) -> bool {
    !source.is_empty()
        && source.len() <= MAX_SOURCE
        && !source.contains(&0)
        && std::str::from_utf8(source).is_ok()
}
impl Store {
    pub fn new(profile: &Path) -> Self {
        Self {
            profile: profile.to_owned(),
        }
    }
    pub fn profile(&self) -> &Path {
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
    pub fn load(&self, key: &[u8; 32]) -> io::Result<Option<Cached>> {
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
    pub fn load_blob(&self, key: &[u8; 32]) -> io::Result<Option<Vec<u8>>> {
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
        if regular_metadata(&file)?.len() > MAX_SOURCE as u64 {
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
    /// Call only after successful download/signature validation. Write failure preserves online execution; never silently evict useful caches.
    pub fn put(
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
    pub fn put_blob(
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
            let filename = entry.file_name();
            if filename == ".lock" {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if !metadata.is_file()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
            // Charge tmp/unknown files too so crash leftovers cannot cause unbounded cache growth.
            if filename != name {
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
pub fn directory(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}
pub fn regular(file: &File) -> io::Result<()> {
    regular_metadata(file).map(|_| ())
}
fn regular_metadata(file: &File) -> io::Result<std::fs::Metadata> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(metadata)
}
#[cfg(test)]
#[path = "art_source_cache_tests.rs"]
mod tests;
