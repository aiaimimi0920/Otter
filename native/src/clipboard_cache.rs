//! 原生剪贴板 PNG 暂存与发布；发布前可取消，发布后文件保留，不跟随窗口析构。
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
/// 单轮清理的哈希读取上限；磁盘配额由 cache_policy 决定。
pub const MAX_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 4096;
#[path = "clipboard_cache_cleanup.rs"]
mod cleanup;
#[path = "clipboard_cache_eviction.rs"]
mod eviction;
#[path = "clipboard_cache_hdr.rs"]
mod hdr;
#[path = "clipboard_cache_receipt.rs"]
mod receipt;
pub use hdr::stage as stage_hdr;
pub struct Prepared {
    staged: crate::session_file_write::Staged,
    stem: String,
    receipt: receipt::Draft,
    // 排他文件句柄覆盖准入、暂存直到发布；不同进程不能同时消耗同一剩余容量。
    _lock: File,
}
impl Prepared {
    /// 调用方须在主线程检查请求 owner/文档/取消；之后即使剪贴板部分失败也保留文件。
    pub fn commit(self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        self.publish(cancel, false).map(|(path, _)| path)
    }
    /// 文档原图在缓存锁释放前获得只读租约；租约存活时拒绝写入与删除共享。
    pub(crate) fn commit_leased(self, cancel: &AtomicBool) -> io::Result<(PathBuf, File)> {
        let (path, lease) = self.publish(cancel, true)?;
        Ok((
            path,
            lease.ok_or_else(|| io::Error::other("Missing original file lease"))?,
        ))
    }
    fn publish(self, cancel: &AtomicBool, leased: bool) -> io::Result<(PathBuf, Option<File>)> {
        let path = self
            .staged
            .commit_incremented(&self.stem, Some("png"), cancel)?;
        let lease = if leased {
            Some(
                OpenOptions::new()
                    .read(true)
                    .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
                    .open(&path)
                    .map_err(|error| {
                        io::Error::other(format!(
                            "Clipboard PNG retained at {}; original lease failed: {error}",
                            path.display()
                        ))
                    })?,
            )
        } else {
            None
        };
        // PNG 已发布后不再按迟到取消撤销；记录失败也保留 PNG，不能假装整个事务回滚。
        self.receipt.publish(&path).map_err(|error| {
            io::Error::other(format!(
                "Clipboard PNG retained at {}; ownership receipt failed: {error}",
                path.display()
            ))
        })?;
        Ok((ordinary_path(path)?, lease))
    }
}

// canonicalize 产生 verbatim 前缀；只转换已知磁盘/UNC 形式，不放宽设备命名空间准入。
fn ordinary_path(path: PathBuf) -> io::Result<PathBuf> {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Component, Prefix},
    };
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing cache filesystem prefix",
        ));
    };
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    match prefix.kind() {
        Prefix::VerbatimDisk(_) => Ok(PathBuf::from(OsString::from_wide(&units[4..]))),
        Prefix::VerbatimUNC(_, _) => {
            let mut ordinary = vec![b'\\' as u16; 2];
            ordinary.extend_from_slice(&units[8..]);
            Ok(PathBuf::from(OsString::from_wide(&ordinary)))
        }
        Prefix::Disk(_) | Prefix::UNC(_, _) => Ok(path),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported cache filesystem prefix",
        )),
    }
}

pub fn stage(
    profile: &Path,
    stem: &str,
    png: crate::pixels::Pixels,
    cancel: &AtomicBool,
) -> io::Result<Prepared> {
    stage_with_policy(profile, stem, png, cancel, Default::default())
}
pub fn stage_with_policy(
    profile: &Path,
    stem: &str,
    png: crate::pixels::Pixels,
    cancel: &AtomicBool,
    policy: crate::cache_policy::Policy,
) -> io::Result<Prepared> {
    check(cancel)?;
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid clipboard PNG signature",
        ));
    }
    let (directory, lock) = admit_staging(profile, stem, png.len() as u64, cancel, policy)?;
    let receipt = receipt::Draft::new(&png)?;
    let staged = crate::session_file_write::Staged::prepare_bytes(
        &directory.join("pending.png"),
        png,
        cancel,
    )?;
    Ok(Prepared {
        staged,
        stem: stem.into(),
        receipt,
        _lock: lock,
    })
}
fn admit_staging(
    profile: &Path,
    stem: &str,
    incoming: u64,
    cancel: &AtomicBool,
    policy: crate::cache_policy::Policy,
) -> io::Result<(PathBuf, File)> {
    let policy = policy
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    check(cancel)?;
    if !profile.is_absolute()
        || stem.len() > crate::file_naming::MAX_CONTEXT_BYTES
        || incoming > 257 * 1024 * 1024
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid clipboard cache input",
        ));
    }
    // 这里只验证所属 PNG 编码器结果的封装，不把任意外部输入当作完整 PNG 解码。
    if policy.temp_cache_max_bytes != 0
        && incoming + receipt::MAX_BYTES as u64 > policy.temp_cache_max_bytes
    {
        return Err(full());
    }
    let directory = profile.join("clipboard_cache");
    for path in [profile, directory.as_path()] {
        crate::profile_directory::validate(path)?;
    }
    fs::create_dir_all(&directory)?;
    for path in [profile, directory.as_path()] {
        crate::profile_directory::validate(path)?;
    }
    let lock = acquire_lock(&directory.join("cache.lock"), cancel)?;
    let metadata = lock.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid clipboard cache lock",
        ));
    }
    let cleaned = cleanup::run_with_retention(
        &directory,
        std::time::SystemTime::now(),
        cancel,
        policy.temp_cache_retention_days,
    )?;
    let mut hashed = cleaned.hashed_bytes;
    eviction::run(
        &directory,
        incoming + receipt::MAX_BYTES as u64,
        policy.temp_cache_max_bytes,
        &mut hashed,
        cancel,
    )?;
    admit(
        &directory,
        incoming + receipt::MAX_BYTES as u64,
        cancel,
        policy.temp_cache_max_bytes,
    )?;
    Ok((directory, lock))
}
fn admit(directory: &Path, incoming: u64, cancel: &AtomicBool, max_bytes: u64) -> io::Result<()> {
    let mut total = incoming;
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        check(cancel)?;
        if index >= MAX_ENTRIES - 2 {
            return Err(full());
        }
        let metadata = fs::symlink_metadata(entry?.path())?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unexpected clipboard cache entry",
            ));
        }
        total = total.checked_add(metadata.len()).ok_or_else(full)?;
        if max_bytes != 0 && total > max_bytes {
            return Err(full());
        }
    }
    Ok(())
}
fn full() -> io::Error {
    io::Error::other("Clipboard cache capacity exhausted; files retained")
}
fn acquire_lock(path: &Path, cancel: &AtomicBool) -> io::Result<File> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        check(cancel)?;
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)
        {
            Ok(file) => return Ok(file),
            Err(error)
                if matches!(error.raw_os_error(), Some(32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                // 仅后台任务争锁时短暂等待，给已取消结果的 UI 回收机会；不增加空闲轮询。
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Clipboard cache cancelled",
        ))
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "clipboard_cache_tests.rs"]
mod tests;
