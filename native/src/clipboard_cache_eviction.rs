//! 配额压力淘汰；有限索引按记录时间排序，再通过共用句柄核验删除，不信任排序快照。
use super::{cleanup, receipt, MAX_BYTES, MAX_ENTRIES};
use std::{
    ffi::OsString,
    fs, io,
    os::windows::fs::MetadataExt,
    path::Path,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
struct Candidate {
    name: OsString,
    record: receipt::Receipt,
}

pub(super) fn run(
    directory: &Path,
    incoming: u64,
    max: u64,
    hashed: &mut u64,
    cancel: &AtomicBool,
) -> io::Result<()> {
    super::check(cancel)?;
    if max == 0 {
        return Ok(());
    }
    // 单次输出本身超限时不能为注定失败的请求清空旧缓存。
    if incoming > max {
        return Err(super::full());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let mut total = 0u64;
    let mut names = Vec::new();
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        super::check(cancel)?;
        if index >= MAX_ENTRIES {
            return Err(super::full());
        }
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(io::Error::other("Unexpected clipboard cache entry"));
        }
        total = total.checked_add(metadata.len()).ok_or_else(super::full)?;
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
        names.push(name);
    }
    if total <= max - incoming {
        return Ok(());
    }
    let mut candidates = Vec::new();
    for name in names {
        super::check(cancel)?;
        if let Ok((_, record)) = cleanup::read_record(&directory.join(&name)) {
            if record.created <= now {
                candidates.push(Candidate { name, record });
            }
        }
    }
    let target = (max / 2).min(max - incoming);
    candidates.sort_by(|left, right| {
        left.record
            .created
            .cmp(&right.record.created)
            .then_with(|| left.name.cmp(&right.name))
    });
    for candidate in candidates {
        super::check(cancel)?;
        if total <= target || *hashed >= MAX_BYTES {
            break;
        }
        match cleanup::retire_matching(
            directory,
            &directory.join(&candidate.name),
            |current| current == &candidate.record,
            hashed,
            cancel,
        ) {
            Ok(Some(freed)) => total = total.saturating_sub(freed),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
            _ => {} // 不匹配/忙/损坏/预算不足时保留，不退化为按路径删除。
        }
    }
    if total > max - incoming {
        Err(super::full())
    } else {
        Ok(())
    }
}
