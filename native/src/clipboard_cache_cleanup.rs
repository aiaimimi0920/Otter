//! 仅在缓存排他锁内运行；不信任文件名或记录，身份/内容匹配后通过同一句柄退休。
use super::{receipt, MAX_BYTES, MAX_ENTRIES};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path},
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows::Win32::{
    Foundation::{GENERIC_READ, HANDLE},
    Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, DELETE,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    },
};
#[derive(Default, Debug)]
pub(super) struct Report {
    pub removed: usize,
    pub skipped: usize,
    pub hashed_bytes: u64,
}

#[cfg(test)]
pub(super) fn run(directory: &Path, now: SystemTime, cancel: &AtomicBool) -> io::Result<Report> {
    run_with_retention(directory, now, cancel, 7)
}
pub(super) fn run_with_retention(
    directory: &Path,
    now: SystemTime,
    cancel: &AtomicBool,
    days: u32,
) -> io::Result<Report> {
    super::check(cancel)?;
    if days == 0 {
        return Ok(Report::default());
    }
    let retention = Duration::from_secs(u64::from(days) * 86400);
    let now = now
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let mut report = Report::default();
    let mut read_bytes = 0u64;
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        super::check(cancel)?;
        if index >= MAX_ENTRIES {
            break;
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name
            .to_str()
            .and_then(|name| name.strip_prefix(".otter-clipboard-"))
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        if id.len() != 36 || uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        match retire_one(
            directory,
            &entry.path(),
            now,
            retention,
            &mut read_bytes,
            cancel,
        ) {
            Ok(true) => report.removed += 1,
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
            Err(_) => report.skipped += 1, // 保留未知/损坏/忙/已改变的文件；不扩大删除范围。
        }
    }
    report.hashed_bytes = read_bytes;
    Ok(report)
}

fn open(path: &Path) -> io::Result<(File, BY_HANDLE_FILE_INFORMATION)> {
    let file = OpenOptions::new()
        .read(true)
        .access_mode(GENERIC_READ.0 | DELETE.0)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(invalid());
    }
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
        .map_err(io::Error::other)?;
    if info.nNumberOfLinks != 1 {
        return Err(invalid());
    }
    Ok((file, info))
}
fn retire_one(
    directory: &Path,
    record_path: &Path,
    now: u64,
    retention: Duration,
    read_bytes: &mut u64,
    cancel: &AtomicBool,
) -> io::Result<bool> {
    retire_matching(
        directory,
        record_path,
        |record| now.saturating_sub(record.created) > retention.as_secs(),
        read_bytes,
        cancel,
    )
    .map(|removed| removed.is_some())
}
pub(super) fn read_record(record_path: &Path) -> io::Result<(File, receipt::Receipt)> {
    let (mut record_file, _) = open(record_path)?;
    let len = record_file.metadata()?.len();
    if len == 0 || len > receipt::MAX_BYTES as u64 {
        return Err(invalid());
    }
    let mut bytes = [0u8; receipt::MAX_BYTES];
    record_file.read_exact(&mut bytes[..len as usize])?;
    let record: receipt::Receipt =
        serde_json::from_slice(&bytes[..len as usize]).map_err(|_| invalid())?;
    if record.version != 1
        || record.created == 0
        || record.bytes == 0
        || record.bytes > 257 * 1024 * 1024
        || !valid_name(&record.name)
    {
        return Err(invalid());
    }
    Ok((record_file, record))
}
pub(super) fn retire_matching(
    directory: &Path,
    record_path: &Path,
    accept: impl FnOnce(&receipt::Receipt) -> bool,
    read_bytes: &mut u64,
    cancel: &AtomicBool,
) -> io::Result<Option<u64>> {
    let (record_file, record) = read_record(record_path)?;
    if !accept(&record) {
        return Ok(None);
    }
    let freed = record
        .bytes
        .checked_add(record_file.metadata()?.len())
        .ok_or_else(invalid)?;
    let next_total = read_bytes.checked_add(record.bytes).ok_or_else(invalid)?;
    if next_total > MAX_BYTES {
        return Ok(None);
    }
    *read_bytes = next_total;
    let (mut png, info) = open(&directory.join(&record.name))?;
    if info.dwVolumeSerialNumber != record.volume
        || ((u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow)) != record.index
        || png.metadata()?.len() != record.bytes
    {
        return Err(invalid());
    }
    let mut hash = Sha256::new();
    let mut remaining = record.bytes;
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        super::check(cancel)?;
        let length = remaining.min(buffer.len() as u64) as usize;
        png.read_exact(&mut buffer[..length])?;
        hash.update(&buffer[..length]);
        remaining -= length as u64;
    }
    if <[u8; 32]>::from(hash.finalize()) != record.sha256 {
        return Err(invalid());
    }
    super::check(cancel)?;
    // 两句柄都仍拒绝写入/删除共享，不先 close 再按可变路径删除。
    crate::session_file_rename::retire(&png)?;
    // 这一步失败可留下孤立记录，但绝不据此重新按路径删除 PNG。
    crate::session_file_rename::retire(&record_file)?;
    Ok(Some(freed))
}
fn valid_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && name.ends_with(".png")
        && name.chars().count() <= 124
        && !name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, ':' | '/' | '\\'))
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Cache ownership verification failed",
    )
}
