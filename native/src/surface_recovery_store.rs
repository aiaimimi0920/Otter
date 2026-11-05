//! 有界、只增不覆盖的恢复证据文件；不删除旧记录，不把半写文件当成有效恢复信息。
use crate::surface_recovery_record::{self as record, Record, MAX_BYTES};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
const MAX_FILES: usize = 128;

/// 调用方在后台执行；新文件失败可残留，原文件永远不覆盖。目录满时显式失败。
pub fn save(root: &Path, records: &[Record]) -> io::Result<PathBuf> {
    if records.is_empty() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    save_bytes(root, "surface-recovery", &record::encode(records)?)
}
/// 动作提示与附件提示使用独立目录/格式，不能被旧附件读取器误解释。
pub fn save_events(
    root: &Path,
    records: &[crate::surface_event_record::Record],
) -> io::Result<PathBuf> {
    if records.is_empty() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    save_bytes(
        root,
        "surface-event-recovery",
        &crate::surface_event_record::encode(records)?,
    )
}
fn save_bytes(root: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    crate::settings_profile::select(None, Some(root.as_os_str()), None, false)
        .map_err(io::Error::other)?;
    let directory = root.join(name);
    fs::create_dir_all(&directory)?;
    // 固定槽位配合 create_new，多个进程并发也不能越过配额；残缺文件仍占槽。
    for index in 0..MAX_FILES {
        let path = directory.join(format!("surface-{index:03}.json"));
        let mut file = match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        file.write_all(bytes)?;
        file.sync_all()?;
        return Ok(path);
    }
    Err(io::Error::other("Surface recovery directory is full"))
}
/// 仅返回提示，不执行任何网络动作。截断、扩展字段、超量输入均拒绝。
pub fn load(path: &Path) -> io::Result<Vec<Record>> {
    record::decode(&load_bytes(path, MAX_BYTES)?)
}
pub fn load_events(path: &Path) -> io::Result<Vec<crate::surface_event_record::Record>> {
    crate::surface_event_record::decode(&load_bytes(path, crate::surface_event_record::MAX_BYTES)?)
}
fn load_bytes(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    // 文件选择器之外的调用也不能把设备管道或 ADS 当作普通恢复文件打开。
    crate::settings_profile::select(None, Some(path.as_os_str()), None, false)
        .map_err(io::Error::other)?;
    if path
        .file_name()
        .is_none_or(|name| name.encode_wide().any(|c| c == 0 || c == b':' as u16))
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}
