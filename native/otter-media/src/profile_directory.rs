//! Native profile-directory admission; validate is read-only, while pin creates and pins non-reparse ancestors until the owner releases them.
use std::{io, os::windows::fs::MetadataExt, path::Path};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

/// Pin ancestors level by level; reject reparse points and directory renames while held, shared by independent profile owners.
pub fn pin(root: &Path) -> io::Result<Vec<std::fs::File>> {
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
pub fn validate(path: &Path) -> io::Result<()> {
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
