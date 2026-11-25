//! Same-directory device-identity commit; temporary files contain only protected documents, without deleting old files before replacement.
use crate::device_identity::{Identity, MAX_DOCUMENT};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
        System::Com::CoCreateGuid,
    },
};

struct Temporary {
    file: Option<File>,
    path: PathBuf,
    committed: bool,
}
impl Drop for Temporary {
    fn drop(&mut self) {
        self.file.take();
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub fn write(destination: &Path, bytes: &[u8], replace: bool) -> io::Result<()> {
    if bytes.len() > MAX_DOCUMENT
        || Identity::parse(bytes)
            .map_err(|_| io::ErrorKind::InvalidData)?
            .needs_migration()
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let parent = destination.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let guid = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
    let path = parent.join(format!(".device-identity-{guid:?}.tmp"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    let mut staged = Temporary {
        file: Some(file),
        path,
        committed: false,
    };
    staged.file.as_mut().unwrap().write_all(bytes)?;
    staged.file.as_ref().unwrap().sync_all()?;
    staged.file.take();
    let source: Vec<u16> = staged.path.as_os_str().encode_wide().chain([0]).collect();
    let target: Vec<u16> = destination.as_os_str().encode_wide().chain([0]).collect();
    let flags = if replace {
        MOVEFILE_WRITE_THROUGH | MOVEFILE_REPLACE_EXISTING
    } else {
        MOVEFILE_WRITE_THROUGH
    };
    // No cross-volume copy/delete; failure never actively deletes destination, and crashes may leave independent tmp files.
    unsafe { MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(target.as_ptr()), flags) }
        .map_err(io::Error::other)?;
    staged.committed = true;
    Ok(())
}
