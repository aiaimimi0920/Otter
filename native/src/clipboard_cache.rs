//! Native clipboard PNG staging/publication; cancellable before publication, persistent afterward, independent of window destruction.
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
/// Hash-read limit for one cleanup pass; cache_policy determines disk quota.
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
    // The exclusive file handle spans admission, staging, and publication; processes cannot simultaneously consume the same remaining capacity.
    _lock: File,
}
impl Prepared {
    /// Drag-out delivers only a newly generated copy; hold the cache lock until Shell finishes so it can move that copy, not the user's original.
    pub(crate) fn commit_drag(self, cancel: &AtomicBool) -> io::Result<(PathBuf, File)> {
        let lock = self._lock.try_clone()?;
        let path = self.commit(cancel)?;
        Ok((path, lock))
    }
    /// Callers must check request owner/document/cancellation on the main thread; afterward retain the file even if clipboard publication partially fails.
    pub fn commit(self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        self.publish(cancel, false).map(|(path, _)| path)
    }
    /// The document original obtains a read-only lease before the cache lock releases; deny write/delete sharing for the lease lifetime.
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
        let file = receipt::open_published(&path).map_err(|error| {
            let step = if leased {
                "original lease"
            } else {
                "ownership receipt"
            };
            io::Error::other(format!(
                "Clipboard PNG retained at {}; {step} failed: {error}",
                path.display()
            ))
        })?;
        // Receipt verification and the long-lived lease share one read-only handle; post-publication failure still retains the PNG.
        self.receipt.publish(&path, &file).map_err(|error| {
            io::Error::other(format!(
                "Clipboard PNG retained at {}; ownership receipt failed: {error}",
                path.display()
            ))
        })?;
        Ok((ordinary_path(path)?, leased.then_some(file)))
    }
}

// canonicalize produces verbatim prefixes; convert only known disk/UNC forms without widening device-namespace admission.
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
    // Validate only the owning PNG encoder's container here; do not treat arbitrary external input as a fully decoded PNG.
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
    let incoming_total = incoming + receipt::MAX_BYTES as u64;
    eviction::admit_staging(&directory, incoming_total, policy, cancel)?;
    Ok((directory, lock))
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
                // Wait briefly only during background lock contention, allowing UI to reclaim cancelled results; no idle polling.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "clipboard_cache_tests.rs"]
mod tests;
