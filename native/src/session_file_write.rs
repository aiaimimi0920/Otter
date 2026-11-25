//! Same-directory phased Save As commits only new files, without fake CAS for overwriting existing sessions.
use crate::{session_document::decode, session_write::Candidate};
use std::os::windows::ffi::OsStrExt;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH},
        System::Com::CoCreateGuid,
    },
};

pub struct Staged {
    file: Option<File>,
    temporary: PathBuf,
    destination: PathBuf,
    committed: bool,
    bytes: Option<crate::pixels::Pixels>,
    revisions: Option<(u64, u64)>,
}
impl Staged {
    /// Callers explicitly select targets; write only this unique temporary file, and cancellation/failure never touches existing targets.
    pub fn prepare(
        destination: &Path,
        candidate: &Candidate,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        check(cancel)?;
        if candidate.expected_revision.checked_add(1) != Some(candidate.revision) {
            return Err(invalid("Invalid session candidate revision"));
        }
        if !crate::session_write::is_verified(candidate) {
            let document =
                decode(&candidate.bytes).map_err(|_| invalid("Invalid session candidate"))?;
            if document.revision != candidate.revision {
                return Err(invalid("Invalid session candidate revision"));
            }
        }
        let mut staged = Self::write_bytes(destination, &candidate.bytes, cancel)?;
        staged.bytes = Some(candidate.bytes.clone());
        staged.revisions = Some((candidate.expected_revision, candidate.revision));
        Ok(staged)
    }
    /// Immutable encoded results already validated by owning formats, without session-candidate identity binding.
    pub fn prepare_bytes(
        destination: &Path,
        bytes: crate::pixels::Pixels,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        Self::write_bytes(destination, &bytes, cancel)
    }
    fn write_bytes(destination: &Path, bytes: &[u8], cancel: &AtomicBool) -> io::Result<Self> {
        let size = Some(bytes.len());
        let mut staged = crate::session_file_trace::run("staging-open", size, || {
            Self::prepare_file(destination, cancel)
        })?;
        crate::session_file_trace::run("write", size, || {
            for chunk in bytes.chunks(64 * 1024) {
                check(cancel)?;
                staged.file.as_mut().unwrap().write_all(chunk)?;
            }
            Ok::<_, io::Error>(())
        })?;
        crate::session_file_trace::run("sync", size, || staged.file.as_ref().unwrap().sync_all())?;
        check(cancel)?;
        Ok(staged)
    }
    /// Format owners stream into this exclusive file; no full encoded buffers or fabricated session-candidate identity.
    pub fn prepare_stream(
        destination: &Path,
        cancel: &AtomicBool,
        write: impl FnOnce(&mut File, &AtomicBool) -> io::Result<()>,
    ) -> io::Result<Self> {
        let mut staged = crate::session_file_trace::run("staging-open", None, || {
            Self::prepare_file(destination, cancel)
        })?;
        crate::session_file_trace::run("stream-write", None, || {
            write(staged.file.as_mut().unwrap(), cancel)
        })?;
        crate::session_file_trace::run("sync", None, || staged.file.as_ref().unwrap().sync_all())?;
        check(cancel)?;
        Ok(staged)
    }
    fn prepare_file(destination: &Path, cancel: &AtomicBool) -> io::Result<Self> {
        check(cancel)?;
        if !destination.is_absolute() {
            return Err(invalid("Save destination must be absolute"));
        }
        let name = destination
            .file_name()
            .ok_or_else(|| invalid("Missing save filename"))?;
        if name.encode_wide().any(|c| c == 0 || c == b':' as u16) {
            return Err(invalid("Invalid save filename"));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("Missing save directory"))?
            .canonicalize()?;
        if !parent.is_dir() {
            return Err(invalid("Save parent is not a directory"));
        }
        let destination = parent.join(name);
        let guid = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
        let temporary = parent.join(format!(".otter-save-{guid:?}.tmp"));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let staged = Self {
            file: Some(file),
            temporary,
            destination,
            committed: false,
            bytes: None,
            revisions: None,
        };
        Ok(staged)
    }
    /// Bind the same immutable encoded result; coincidentally equal revisions/byte lengths do not prove the same candidate.
    pub fn is_for(&self, candidate: &Candidate) -> bool {
        self.bytes
            .as_ref()
            .is_some_and(|bytes| bytes.same_storage(&candidate.bytes))
            && self.revisions == Some((candidate.expected_revision, candidate.revision))
    }
    /// Callers still recheck UI Receipt before commit; no overwriting or cross-volume copy/delete fallback.
    /// After success, late cancellation never deletes published files; power-loss durability depends on the actual filesystem.
    pub fn commit_new(mut self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        check(cancel)?;
        self.file.take();
        let source: Vec<u16> = self
            .temporary
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let target: Vec<u16> = self
            .destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        check(cancel)?;
        crate::session_file_trace::run("publish-new", None, || unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_WRITE_THROUGH,
            )
        })
        .map_err(io::Error::other)?;
        self.committed = true;
        Ok(self.destination.clone())
    }

    /// Incremental publication of encoded resources reuses the same staged file without rewriting content for name conflicts.
    /// Only resources without session-revision bindings use this; sessions still publish to the exact user-confirmed path.
    pub fn commit_incremented(
        mut self,
        stem: &str,
        extension: Option<&str>,
        cancel: &AtomicBool,
    ) -> io::Result<PathBuf> {
        check(cancel)?;
        if self.revisions.is_some() {
            return Err(invalid("Session candidates require an exact destination"));
        }
        self.file.take();
        let source: Vec<u16> = self
            .temporary
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect();
        let parent = self
            .destination
            .parent()
            .ok_or_else(|| invalid("Missing resource directory"))?;
        let (_, destination) =
            crate::file_naming::allocate::allocate_with(parent, stem, extension, cancel, |path| {
                let target: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
                check(cancel)?;
                // Set neither REPLACE_EXISTING nor COPY_ALLOWED; always same-volume no-overwrite rename.
                unsafe {
                    MoveFileExW(
                        PCWSTR(source.as_ptr()),
                        PCWSTR(target.as_ptr()),
                        MOVEFILE_WRITE_THROUGH,
                    )
                }
                .map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xffff))
            })?;
        self.committed = true;
        Ok(destination)
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        self.file.take();
        if !self.committed {
            // Clean only paths this object successfully created with create_new; never delete targets or scan whole directories.
            let _ = std::fs::remove_file(&self.temporary);
        }
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Session write cancelled",
        ))
    } else {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_file_write_tests.rs"]
mod tests;
