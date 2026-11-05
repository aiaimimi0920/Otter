//! 在既有 DELETE 权限句柄上执行不覆盖的重命名；不按来源路径再次打开文件。
use std::{
    fs::File,
    io,
    os::windows::{ffi::OsStrExt, io::AsRawHandle},
    path::Path,
};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO},
};

pub(crate) fn retire(file: &File) -> io::Result<()> {
    use windows::Win32::Storage::FileSystem::{FileDispositionInfo, FILE_DISPOSITION_INFO};
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&info as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of_val(&info) as u32,
        )
    }
    .map_err(io::Error::other)
}

pub(crate) fn rename_new(file: &File, destination: &Path) -> io::Result<()> {
    if !destination.is_absolute()
        || destination
            .file_name()
            .is_none_or(|name| name.encode_wide().any(|c| c == 0 || c == b':' as u16))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid recovery destination",
        ));
    }
    let name: Vec<u16> = destination.as_os_str().encode_wide().take(32768).collect();
    if name.len() > 32767 || name.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid recovery path length or NUL",
        ));
    }
    let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    let size = (offset + (name.len() + 1) * 2).max(std::mem::size_of::<FILE_RENAME_INFO>());
    // usize 缓冲保证 HANDLE 对齐；尾部 WCHAR 使用 SDK 字段偏移，不假设结构体无填充。
    assert!(std::mem::align_of::<usize>() >= std::mem::align_of::<FILE_RENAME_INFO>());
    let mut storage = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).FileNameLength = (name.len() * 2) as u32;
        // 零初始化保留 ReplaceIfExists=false 与 RootDirectory=NULL；目标不存在才成功。
        let target = storage.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), target, name.len());
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileRenameInfo,
            storage.as_ptr().cast(),
            size as u32,
        )
    }
    .map_err(io::Error::other)
}
