//! 显式选择的会话文件：worker 中有界只读，不从 JSON 推导资源目录权限。
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
/// 显式文件导入共享同一句柄/路径准入，调用方必须提供自己的容量上限。
pub(in crate::desktop) fn read_limited(
    path: &Path,
    cancel: &AtomicBool,
    limit: usize,
) -> io::Result<Vec<u8>> {
    check(cancel)?;
    // 禁止设备命名空间；只接受调用方提供的普通绝对文件路径。
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

fn read_file(mut file: impl Read, length: usize, cancel: &AtomicBool) -> io::Result<Vec<u8>> {
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
/// 仅原路径缺失时读取唯一恢复副本；不移动文件，来源标记交给发布层提示。
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
            // 查找期间原路径重新出现则优先正常文件，不悄悄回退到旧版本。
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
