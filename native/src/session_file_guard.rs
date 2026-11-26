//! Read-only baseline lease for replacement preparation; deny write/delete sharing during full-byte checks, without replacement commit/fake CAS.
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::{
    Foundation::{GENERIC_READ, HANDLE},
    Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, DELETE,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub volume: u32,
    pub index: u64,
}
pub struct Guard {
    _file: File,
    identity: Identity,
}
impl Guard {
    /// Background only; no truncation/creation/source mutation. After lease release, replacement races must be handled again;
    /// drop-then-rename is not atomic conditional update, and sharing restrictions are not a security boundary for malicious directories.
    pub fn open(path: &Path, expected: &[u8], cancel: &AtomicBool) -> io::Result<Self> {
        Self::open_access(path, expected, cancel, GENERIC_READ.0)
    }
    fn open_access(
        path: &Path,
        expected: &[u8],
        cancel: &AtomicBool,
        access: u32,
    ) -> io::Result<Self> {
        check(cancel)?;
        if !path.is_absolute()
            || expected.is_empty()
            || expected.len() > crate::session_document::MAX_SESSION_BYTES
            || path
                .file_name()
                .is_none_or(|name| name.encode_wide().any(|c| c == 0 || c == b':' as u16))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid session baseline path or size",
            ));
        }
        let (mut file, identity) =
            crate::session_file_trace::run("baseline-open", Some(expected.len()), || {
                let file = OpenOptions::new()
                    .read(true)
                    .access_mode(access)
                    .share_mode(FILE_SHARE_READ.0)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
                    .open(path)?;
                let metadata = file.metadata()?;
                if !metadata.is_file()
                    || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Session baseline is not a regular file",
                    ));
                }
                if metadata.len() != expected.len() as u64 {
                    return Err(conflict());
                }
                let mut information = BY_HANDLE_FILE_INFORMATION::default();
                unsafe {
                    GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information)
                }
                .map_err(io::Error::other)?;
                let identity = Identity {
                    volume: information.dwVolumeSerialNumber,
                    index: (u64::from(information.nFileIndexHigh) << 32)
                        | u64::from(information.nFileIndexLow),
                };
                Ok::<_, io::Error>((file, identity))
            })?;
        crate::session_file_trace::run("baseline-read", Some(expected.len()), || {
            // Performance audit optimization (Tier 1.2):
            // Read baseline session files in 64 KiB chunks instead of 8 KiB, cutting `ReadFile` syscall overhead 8x on
            // large multi-sticker workspace checkpoints (~60 MiB: ~7,680 syscalls -> ~960 syscalls).
            let mut buffer = vec![0u8; 64 * 1024];
            for chunk in expected.chunks(buffer.len()) {
                check(cancel)?;
                file.read_exact(&mut buffer[..chunk.len()])?;
                if &buffer[..chunk.len()] != chunk {
                    return Err(conflict());
                }
            }
            check(cancel)
        })?;
        Ok(Self {
            _file: file,
            identity,
        })
    }
    pub fn identity(&self) -> Identity {
        self.identity
    }
}

/// Explicitly promote to a rename-capable baseline handle; ordinary read-only Guard exposes no move capability.
pub struct Movable(Guard);
impl Movable {
    pub fn open(path: &Path, expected: &[u8], cancel: &AtomicBool) -> io::Result<Self> {
        Guard::open_access(path, expected, cancel, GENERIC_READ.0 | DELETE.0).map(Self)
    }
    pub fn identity(&self) -> Identity {
        self.0.identity()
    }
    /// Never overwrite targets; the same handle retains sharing restrictions without reopening sources by path.
    /// This step only moves old files, not publishes new content; callers own recovery paths/interruption-recovery protocols.
    pub fn rename_new(&self, destination: &Path) -> io::Result<()> {
        crate::session_file_rename::rename_new(&self.0._file, destination)
    }
    /// Retire the same old file only after new content publication; never close handles then delete through mutable paths.
    pub(crate) fn retire(&self) -> io::Result<()> {
        crate::session_file_rename::retire(&self.0._file)
    }
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Session baseline check cancelled",
        ))
    } else {
        Ok(())
    }
}
fn conflict() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Session file differs from retained baseline",
    )
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_file_guard_tests.rs"]
mod tests;
