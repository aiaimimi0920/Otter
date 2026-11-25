//! Quota-pressure eviction and final admission revalidation; verify deletion through shared handles, not sorted snapshots.
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
struct Snapshot {
    total: u128,
    records: Vec<OsString>,
}
impl Snapshot {
    // The exclusive lock constrains only cooperating code in this process; final admission scans must remeasure external changes.
    fn scan(directory: &Path, limit: usize, cancel: &AtomicBool) -> io::Result<Self> {
        let mut total = 0u128;
        let mut records = Vec::new();
        for (index, entry) in fs::read_dir(directory)?.enumerate() {
            super::check(cancel)?;
            if index >= limit {
                return Err(super::full());
            }
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if !metadata.is_file()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unexpected clipboard cache entry",
                ));
            }
            total = total
                .checked_add(u128::from(metadata.len()))
                .ok_or_else(super::full)?;
            let name = entry.file_name();
            if is_record_name(&name) {
                records.push(name);
            }
        }
        Ok(Self { total, records })
    }
}
fn is_record_name(name: &OsString) -> bool {
    name.to_str()
        .and_then(|s| s.strip_prefix(".otter-clipboard-"))
        .and_then(|s| s.strip_suffix(".json"))
        .is_some_and(|id| id.len() == 36 && uuid::Uuid::parse_str(id).is_ok())
}
pub(super) fn admit_staging(
    directory: &Path,
    incoming: u64,
    policy: crate::cache_policy::Policy,
    cancel: &AtomicBool,
) -> io::Result<()> {
    super::check(cancel)?;
    let max = policy.temp_cache_max_bytes;
    if max != 0 && incoming > max {
        return Err(super::full());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let mut snapshot = Snapshot::scan(directory, MAX_ENTRIES.saturating_mul(2), cancel)?;
    let mut hashed = 0u64;
    if policy.temp_cache_retention_days > 0 {
        let retention = u64::from(policy.temp_cache_retention_days) * 86400;
        let mut kept = Vec::with_capacity(snapshot.records.len());
        for name in snapshot.records.into_iter().take(MAX_ENTRIES) {
            super::check(cancel)?;
            match cleanup::retire_matching(
                directory,
                &directory.join(&name),
                |record| now.saturating_sub(record.created) > retention,
                &mut hashed,
                cancel,
            ) {
                Ok(Some(freed)) => {
                    snapshot.total = snapshot.total.saturating_sub(u128::from(freed));
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
                _ => kept.push(name),
            }
        }
        snapshot.records = kept;
    }
    if max != 0 && snapshot.total > u128::from(max - incoming) {
        evict_from_snapshot(
            directory,
            &mut snapshot,
            incoming,
            max,
            now,
            &mut hashed,
            cancel,
        )?;
    }
    // External files may change during eviction; old snapshots select candidates but cannot prove current admission capacity.
    let current = Snapshot::scan(directory, MAX_ENTRIES - 2, cancel)?;
    let final_total = u64::try_from(current.total)
        .ok()
        .and_then(|total| total.checked_add(incoming))
        .ok_or_else(super::full)?;
    if max != 0 && final_total > max {
        return Err(super::full());
    }
    Ok(())
}
#[cfg(test)]
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
    if incoming > max {
        return Err(super::full());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let mut snapshot = Snapshot::scan(directory, MAX_ENTRIES, cancel)?;
    if snapshot.total <= u128::from(max - incoming) {
        return Ok(());
    }
    evict_from_snapshot(directory, &mut snapshot, incoming, max, now, hashed, cancel)
}
fn evict_from_snapshot(
    directory: &Path,
    snapshot: &mut Snapshot,
    incoming: u64,
    max: u64,
    now: u64,
    hashed: &mut u64,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let mut candidates = Vec::new();
    for name in &snapshot.records {
        super::check(cancel)?;
        if let Ok((_, record)) = cleanup::read_record(&directory.join(name)) {
            if record.created <= now {
                candidates.push(Candidate {
                    name: name.clone(),
                    record,
                });
            }
        }
    }
    let target = u128::from((max / 2).min(max - incoming));
    candidates.sort_by(|left, right| {
        left.record
            .created
            .cmp(&right.record.created)
            .then_with(|| left.name.cmp(&right.name))
    });
    for candidate in candidates {
        super::check(cancel)?;
        if snapshot.total <= target || *hashed >= MAX_BYTES {
            break;
        }
        match cleanup::retire_matching(
            directory,
            &directory.join(&candidate.name),
            |current| current == &candidate.record,
            hashed,
            cancel,
        ) {
            Ok(Some(freed)) => {
                snapshot.total = snapshot.total.saturating_sub(u128::from(freed));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
            _ => {}
        }
    }
    if snapshot.total > u128::from(max - incoming) {
        Err(super::full())
    } else {
        Ok(())
    }
}
