//! 原生 profile 目录准入；validate 只读，pin 创建并固定非重解析祖先直到 owner 释放。
use std::{io, os::windows::fs::MetadataExt, path::Path};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

/// 逐层固定祖先；持有期间拒绝重解析点和目录重命名，供独立 profile owner 共用。
pub(crate) fn pin(root: &Path) -> io::Result<Vec<std::fs::File>> {
    use std::{
        fs::OpenOptions,
        os::windows::fs::OpenOptionsExt,
        path::{Component, PathBuf, Prefix},
    };
    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Profile requires non-reparse absolute ancestors",
        )
    };
    if !root.is_absolute()
        || root.components().count() > 128
        || !matches!(root.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(_, _)))
    {
        return Err(invalid());
    }
    let mut path = PathBuf::new();
    let mut directories = Vec::new();
    for component in root.components() {
        match component {
            Component::Prefix(_) => {
                path.push(component);
                continue;
            }
            Component::RootDir | Component::Normal(_) => path.push(component),
            _ => return Err(invalid()),
        }
        if matches!(component, Component::Normal(_)) {
            if let Err(error) = std::fs::create_dir(&path) {
                if error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(error);
                }
            }
        }
        let directory = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(&path)?;
        let metadata = directory.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(invalid());
        }
        directories.push(directory);
    }
    Ok(directories)
}
pub(crate) fn validate(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata)
            if !metadata.is_dir()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 =>
        {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Native profile root must be a non-reparse directory",
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
