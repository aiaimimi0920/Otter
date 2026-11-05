//! 显式离线凭证与启动时钟高水位；独占有界槽、原子替换，不保存私钥/token或用户输入。
//! 高水位保护合作进程重启，不承诺抵抗同用户恶意回滚整个profile/系统时钟。
use crate::{
    art_source_cache::{directory, regular},
    art_source_cache_file,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use windows::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
const MAX_BYTES: usize = 32 * 1024;
const MAX_ENTRIES: usize = 32;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    schema_version: u32,
    observed_wall_ms: u64,
    pub ticket: Option<crate::art_execution_grant::wire::Envelope>,
}
pub(crate) struct Slot {
    root: PathBuf,
    path: PathBuf,
    _lock: File,
}
impl Slot {
    /// key只来自当前host的origin/art/issuer/设备公钥；不是远端文件名。
    pub(crate) fn open(
        profile: &Path,
        origin: &str,
        art: &str,
        pin: &crate::live_relay_http::art_prepare::Pin,
        device_key: &str,
    ) -> io::Result<Self> {
        let key = serde_json::to_vec(&(origin, art, &pin.key_id, pin.public_key, device_key))?;
        Self::open_key(profile, &key)
    }
    fn for_origin(profile: &Path, origin: &str) -> io::Result<Self> {
        Self::open_key(profile, &serde_json::to_vec(&("deny-origin", origin))?)
    }
    fn open_key(profile: &Path, key: &[u8]) -> io::Result<Self> {
        if !profile.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        directory(profile)?;
        let root = profile.canonicalize()?.join("art-offline-cache");
        match std::fs::create_dir(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
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
        let path = root.join(format!("{:x}.json", Sha256::digest(key)));
        Ok(Self {
            root,
            path,
            _lock: lock,
        })
    }
    /// 显式配对/续签尝试先减权，远端拒绝或新身份不能在重启后复活旧离线凭证。
    pub(crate) fn invalidate_origin(
        profile: &Path,
        origin: &str,
        now: u64,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<()> {
        match std::fs::symlink_metadata(profile.join("art-offline-cache")) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
            Ok(_) => {}
        }
        Self::for_origin(profile, origin)?.invalidate(now, stopped)
    }
    pub(crate) fn cutoff(profile: &Path, origin: &str) -> io::Result<u64> {
        Ok(Self::for_origin(profile, origin)?
            .read()?
            .map_or(0, |r| r.observed_wall_ms))
    }
    fn read(&self) -> io::Result<Option<Record>> {
        let file = match OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        regular(&file)?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut bytes = Vec::new();
        file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
        let record: Record = serde_json::from_slice(&bytes)?;
        if bytes.len() > MAX_BYTES || record.schema_version != 1 || record.observed_wall_ms == 0 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Some(record))
    }
    /// 在线尝试先写减权记录。403/停用/版本变化/网络失败后均不自动复活旧离线包。
    pub(crate) fn invalidate(&self, now: u64, stopped: &impl Fn() -> bool) -> io::Result<()> {
        let prior = self.read()?;
        self.write(
            Record {
                schema_version: 1,
                observed_wall_ms: prior.map_or(now, |r| r.observed_wall_ms.max(now)),
                ticket: None,
            },
            stopped,
        )
    }
    pub(crate) fn save(
        &self,
        ticket: crate::art_execution_grant::wire::Envelope,
        now: u64,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<()> {
        if self.read()?.is_some_and(|r| now < r.observed_wall_ms) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.write(
            Record {
                schema_version: 1,
                observed_wall_ms: now,
                ticket: Some(ticket),
            },
            stopped,
        )
    }
    /// 先持久化启动高水位再返回凭证；写入失败不能执行。执行中另由原Permit/本地租约单调检查。
    pub(crate) fn take(&self, now: u64, stopped: &impl Fn() -> bool) -> io::Result<Vec<u8>> {
        let mut record = self.read()?.ok_or(io::ErrorKind::NotFound)?;
        if now < record.observed_wall_ms {
            record.ticket = None;
            self.write(record, stopped)?;
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        if record.ticket.is_none() {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        record.observed_wall_ms = now;
        let bytes = serde_json::to_vec(record.ticket.as_ref().unwrap())?;
        self.write(record, stopped)?;
        Ok(bytes)
    }
    fn write(&self, record: Record, stopped: &impl Fn() -> bool) -> io::Result<()> {
        let bytes = serde_json::to_vec(&record)?;
        if bytes.len() > MAX_BYTES || stopped() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let (mut count, mut total) = (0usize, 0u64);
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_name() == ".lock" {
                continue;
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
                .open(entry.path())?;
            regular(&file)?;
            if entry.path() != self.path {
                count += 1;
                total = total
                    .checked_add(file.metadata()?.len())
                    .ok_or(io::ErrorKind::InvalidData)?;
            }
            if count >= MAX_ENTRIES || total > (MAX_ENTRIES * MAX_BYTES) as u64 {
                return Err(io::ErrorKind::OutOfMemory.into());
            }
        }
        if total + bytes.len() as u64 > (MAX_ENTRIES * MAX_BYTES) as u64 {
            return Err(io::ErrorKind::OutOfMemory.into());
        }
        art_source_cache_file::write(&self.path, &bytes, stopped)
    }
}
#[cfg(test)]
#[path = "art_offline_store_tests.rs"]
mod tests;
