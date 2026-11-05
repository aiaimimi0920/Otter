//! 默认工作区的路径/目录/进程租约；与旧版目录隔离，检查入口绝不发现真实 profile。
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};

pub(super) struct Profile {
    pub(super) path: PathBuf,
    pub(super) restore: bool,
    // 文件先于目录析构；持有期间不允许替换租约文件或重命名任何路径祖先。
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
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
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
        // 不读内容、不猜图片权限；真实有界读取/格式验证仍由 session_job 完成。
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
            _lease: lease,
            _directories: directories,
        })
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
#[path = "workspace_profile_tests.rs"]
mod tests;
