//! Read-only local installation path leases; callers supply their own ACL
//! policy before the next path component is resolved.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, Prefix},
};
use windows::{
    core::PCWSTR,
    Win32::{Foundation::HANDLE, Storage::FileSystem::*},
};

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Untrusted local installation path",
    )
}
pub fn directories(
    path: &Path,
    check_acl: impl Fn(&File, &Path) -> io::Result<()>,
) -> io::Result<Vec<File>> {
    directories_with(path, false, check_acl)
}
pub fn shared_directories(path: &Path) -> io::Result<Vec<File>> {
    directories_with(path, true, |_, _| Ok(()))
}
fn directories_with(
    path: &Path,
    share_leaf_writes: bool,
    check_acl: impl Fn(&File, &Path) -> io::Result<()>,
) -> io::Result<Vec<File>> {
    let components: Vec<_> = path.components().collect();
    let Some(Component::Prefix(prefix)) = components.first() else {
        return Err(invalid());
    };
    let Prefix::Disk(drive) = prefix.kind() else {
        return Err(invalid());
    };
    if !path.is_absolute()
        || components.len() < 3
        || components.len() > 64
        || path
            .to_str()
            .ok_or_else(invalid)?
            .split(['\\', '/'])
            .skip(1)
            .any(|p| matches!(p, "." | ".."))
    {
        return Err(invalid());
    }
    let wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.len() > 32760 || wide.contains(&0) {
        return Err(invalid());
    }
    let root = [u16::from(drive), 58, 92, 0];
    if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != 3 {
        return Err(invalid());
    }
    for component in &components[2..] {
        let Component::Normal(name) = component else {
            return Err(invalid());
        };
        let name = name.to_str().ok_or_else(invalid)?;
        if name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return Err(invalid());
        }
        let stem = name
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end()
            .to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p).is_some_and(|s| {
                matches!(
                    s,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        }) {
            return Err(invalid());
        }
    }
    // Non-leaf ancestors omit FILE_SHARE_DELETE so they cannot be renamed, while
    // sharing writes avoids spurious violations on C:\ or user profile folders.
    let mut held = Vec::new();
    let mut current = std::path::PathBuf::from(format!("{}:\\", drive as char));
    let root = open_directory(&current, FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)?;
    check_acl(&root, &current)?;
    held.push(root);
    let last = components.len() - 1;
    for (index, component) in components.iter().enumerate().skip(2) {
        current.push(component.as_os_str());
        let file = if index == last && !share_leaf_writes {
            directory(&current)?
        } else {
            open_directory(&current, FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)?
        };
        check_acl(&file, &current)?;
        held.push(file);
    }
    Ok(held)
}
pub fn directory(path: &Path) -> io::Result<File> {
    open_directory(path, FILE_SHARE_READ.0)
}
fn open_directory(path: &Path, share: u32) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(share)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    check(&file, true)?;
    Ok(file)
}
pub fn binary(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    check(&file, false)?;
    Ok(file)
}
fn check(file: &File, directory: bool) -> io::Result<()> {
    let metadata = file.metadata()?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }?;
    if metadata.is_dir() != directory
        || (!directory && (!metadata.is_file() || info.nNumberOfLinks != 1))
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || unsafe { GetFileType(HANDLE(file.as_raw_handle())) } != FILE_TYPE_DISK
    {
        return Err(invalid());
    }
    Ok(())
}
