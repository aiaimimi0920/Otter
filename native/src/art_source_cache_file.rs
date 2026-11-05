//! 缓存同目录暂存/原子替换；只清理本次create_new拥有的临时文件，不删除旧目标。
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
};
struct Staged {
    file: Option<File>,
    path: PathBuf,
    committed: bool,
}
impl Drop for Staged {
    fn drop(&mut self) {
        self.file.take();
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub(crate) fn write(
    destination: &Path,
    bytes: &[u8],
    stopped: &impl Fn() -> bool,
) -> io::Result<()> {
    let id = unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(io::Error::other)?;
    let path = destination
        .parent()
        .ok_or(io::ErrorKind::InvalidInput)?
        .join(format!(".source-{id:?}.tmp"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    let mut staged = Staged {
        file: Some(file),
        path,
        committed: false,
    };
    staged.file.as_mut().unwrap().write_all(bytes)?;
    staged.file.as_ref().unwrap().sync_all()?;
    staged.file.take();
    if stopped() {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let source: Vec<u16> = staged.path.as_os_str().encode_wide().chain([0]).collect();
    let target: Vec<u16> = destination.as_os_str().encode_wide().chain([0]).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(io::Error::other)?;
    staged.committed = true;
    Ok(())
}
