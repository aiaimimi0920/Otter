//! Default-workspace path/directory/process leases; isolated from legacy directories, with inspection entries never discovering actual profiles.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use windows::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_OPEN_REPARSE_POINT,
};

pub(super) struct Profile {
    pub(super) path: PathBuf,
    pub(super) restore: bool,
    pub(super) claimed: Option<restart::Checkpoint>,
    // Destroy files before directories; while held, neither replace lease files nor rename any path ancestor.
    _lease: File,
    _directories: Vec<File>,
}
pub(super) fn resolve(enabled: bool, check: bool) -> io::Result<Option<Profile>> {
    if !enabled {
        return Ok(None);
    }
    let override_root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    if check && override_root.is_none() {
        return Err(invalid(
            "--workspace-check requires OTTER_NATIVE_PROFILE_DIR",
        ));
    }
    let local = std::env::var_os("LOCALAPPDATA");
    let selection =
        crate::settings_profile::select(None, override_root.as_deref(), local.as_deref(), check)
            .map_err(invalid)?
            .ok_or_else(|| invalid("Missing native workspace profile"))?;
    Profile::open(selection.path.parent().expect("profile filename")).map(Some)
}
impl Profile {
    pub(super) fn open(root: &Path) -> io::Result<Self> {
        let directories = crate::profile_directory::pin(root)?;
        let path = root.join("workspace.json");
        let lease = Self::lock(root, true)?;
        // Ordinary startup alone may discover a recovery copy.
        let restore = match std::fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                crate::session_file_recovery::find(&path, &AtomicBool::new(false))?.is_some()
            }
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            restore,
            claimed: None,
            _lease: lease,
            _directories: directories,
        })
    }
    fn lock(root: &Path, create: bool) -> io::Result<File> {
        let lease = OpenOptions::new()
            .read(true)
            .write(create)
            .create(create)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join("workspace.lock"))?;
        let metadata = lease.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(invalid(
                "Workspace lease must be a regular non-reparse file",
            ));
        }
        // The exclusive lock must name only this profile, not some other
        // file on the same volume. Inspect the lease itself, never its path.
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe {
            GetFileInformationByHandle(
                windows::Win32::Foundation::HANDLE(lease.as_raw_handle()),
                &mut info,
            )
        }
        .map_err(io::Error::other)?;
        if info.nNumberOfLinks != 1 {
            return Err(invalid("Workspace lease must have a single link"));
        }
        Ok(lease)
    }
}
#[path = "workspace_restart_checkpoint.rs"]
pub(super) mod restart;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
#[path = "workspace_profile_tests.rs"]
mod tests;
