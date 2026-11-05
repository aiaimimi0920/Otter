//! 覆盖准备的只读基线租约；完整字节核对期间拒绝写/删除共享，不提供替换提交或伪 CAS。
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
    /// 后台调用；不截断、不创建、不修改来源。租约释放后必须重新处理替换竞争，
    /// 不能把先 drop 再 rename 称为原子条件更新；共享限制也不是恶意目录的安全边界。
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
            let mut buffer = [0; 8192];
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

/// 显式提升到有重命名权的基线句柄；普通只读 Guard 不暴露移动能力。
pub struct Movable(Guard);
impl Movable {
    pub fn open(path: &Path, expected: &[u8], cancel: &AtomicBool) -> io::Result<Self> {
        Guard::open_access(path, expected, cancel, GENERIC_READ.0 | DELETE.0).map(Self)
    }
    pub fn identity(&self) -> Identity {
        self.0.identity()
    }
    /// 始终不覆盖目标；同一句柄持有共享限制，不重新按来源路径查找文件。
    /// 这一步仅移动旧文件，不发布新内容；调用方必须承担恢复路径及中断恢复协议。
    pub fn rename_new(&self, destination: &Path) -> io::Result<()> {
        crate::session_file_rename::rename_new(&self.0._file, destination)
    }
    /// 仅在新内容已发布后退休同一旧文件；不关闭句柄后按可变路径删除。
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

#[cfg(test)]
#[path = "session_file_guard_tests.rs"]
mod tests;
