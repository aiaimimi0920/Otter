//! Bounded append-only recovery-evidence files without overwrites; never delete old records or treat partial writes as valid recovery information.
use crate::surface_recovery_record::{self as record, Record, MAX_BYTES};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
const MAX_FILES: usize = 128;

/// Callers execute in the background; failed new files may remain, but existing files are never overwritten. Full directories fail explicitly.
pub fn save(root: &Path, records: &[Record]) -> io::Result<PathBuf> {
    if records.is_empty() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    save_bytes(root, "surface-recovery", &record::encode(records)?)
}
/// Action hints and attachment hints use separate directories/formats so old attachment readers cannot misinterpret them.
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
    // Fixed slots with create_new enforce quotas even across concurrent processes; incomplete files still occupy slots.
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
/// Return hints only, without network actions. Reject truncation, extra fields, and oversized input.
pub fn load(path: &Path) -> io::Result<Vec<Record>> {
    record::decode(&load_bytes(path, MAX_BYTES)?)
}
pub fn load_events(path: &Path) -> io::Result<Vec<crate::surface_event_record::Record>> {
    crate::surface_event_record::decode(&load_bytes(path, crate::surface_event_record::MAX_BYTES)?)
}
fn load_bytes(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    // Calls outside the file picker must not open device pipes or ADS as ordinary recovery files either.
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
