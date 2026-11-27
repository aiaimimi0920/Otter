//! Explicitly selected session files: bounded read-only worker access, without deriving resource-directory permissions from JSON.
use std::{
    fs::OpenOptions,
    io::{self, Read},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_SEQUENTIAL_SCAN,
    FILE_SHARE_READ,
};

#[cfg(test)]
pub(super) fn read(path: &Path, cancel: &AtomicBool) -> io::Result<Vec<u8>> {
    read_limited(path, cancel, crate::session_document::MAX_SESSION_BYTES)
}
/// Explicit file imports share the same handle/path admission; callers must supply their own capacity limit.
pub(in crate::desktop) fn read_limited(
    path: &Path,
    cancel: &AtomicBool,
    limit: usize,
) -> io::Result<Vec<u8>> {
    check(cancel)?;
    // Device namespaces are forbidden; accept only ordinary absolute file paths supplied by callers.
    use std::path::{Component, Prefix};
    if !path.is_absolute()
        || !matches!(path.components().next(), Some(Component::Prefix(prefix))
        if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(_, _)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Session requires an ordinary absolute file path",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_SEQUENTIAL_SCAN.0)
        .open(path)?;
    use std::os::windows::io::AsRawHandle;
    if unsafe {
        windows::Win32::Storage::FileSystem::GetFileType(windows::Win32::Foundation::HANDLE(
            file.as_raw_handle(),
        ))
    } != windows::Win32::Storage::FileSystem::FILE_TYPE_DISK
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Session source must be a disk file",
        ));
    }
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Session source must be a regular non-reparse file",
        ));
    }
    let length = usize::try_from(metadata.len())
        .ok()
        .filter(|length| *length <= limit)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Session file exceeds budget"))?;
    read_file(file, length, cancel)
}

pub(in crate::desktop) fn read_file(
    mut file: impl Read,
    length: usize,
    cancel: &AtomicBool,
) -> io::Result<Vec<u8>> {
    check(cancel)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| {
        io::Error::new(
            io::ErrorKind::OutOfMemory,
            "Cannot reserve session file buffer",
        )
    })?;
    let mut buffer = [0u8; 64 * 1024];
    while bytes.len() < length {
        check(cancel)?;
        let amount = (length - bytes.len()).min(buffer.len());
        let count = file.read(&mut buffer[..amount])?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Session file changed during read",
            ));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    check(cancel)?;
    if file.read(&mut buffer[..1])? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Session file grew during read",
        ));
    }
    // The final probe is also blocking I/O; cancellation while it runs must not return bytes.
    check(cancel)?;
    Ok(bytes)
}
/// Read the unique recovery copy only when the original path is missing; do not move files, and pass source markers to publication for disclosure.
pub(super) fn read_recovering(
    path: &Path,
    cancel: &AtomicBool,
) -> io::Result<(Vec<u8>, Option<std::path::PathBuf>)> {
    read_recovering_limited(path, cancel, crate::session_document::MAX_SESSION_BYTES)
}
pub(in crate::desktop) fn read_recovering_limited(
    path: &Path,
    cancel: &AtomicBool,
    limit: usize,
) -> io::Result<(Vec<u8>, Option<std::path::PathBuf>)> {
    match read_limited(path, cancel, limit) {
        Ok(bytes) => Ok((bytes, None)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let Some(recovery) = crate::session_file_recovery::find(path, cancel)? else {
                return Err(error);
            };
            let bytes = read_limited(&recovery, cancel, limit)?;
            // If the original path reappears during lookup, prefer the normal file rather than silently reverting to an old version.
            if path.try_exists()? {
                return read_limited(path, cancel, limit).map(|bytes| (bytes, None));
            }
            Ok((bytes, Some(recovery)))
        }
        Err(error) => Err(error),
    }
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Session file read cancelled",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_file_cancellation_during_eof_probe_rejects_completed_bytes() {
        use std::io::Cursor;

        for content in [b"".as_slice(), b"data".as_slice()] {
            let cancel = AtomicBool::new(false);
            let mut source = Cursor::new(content);
            let read = |buffer: &mut [u8]| {
                let count = source.read(buffer)?;
                if count == 0 {
                    cancel.store(true, Ordering::Release);
                }
                Ok(count)
            };
            struct Reader<F>(F);
            impl<F: FnMut(&mut [u8]) -> io::Result<usize>> Read for Reader<F> {
                fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                    (self.0)(buffer)
                }
            }
            let error = read_file(Reader(read), content.len(), &cancel).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        }
    }
}
