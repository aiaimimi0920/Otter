//! 候选路径租约：固定盘、逐级非 reparse 目录、禁止删除共享；不修改任何 ACL。
use super::invalid;
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

pub(super) fn directories(path: &Path) -> io::Result<Vec<File>> {
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
    let mut held = Vec::new();
    // 固定卷根是 OS 信任锚；后续每级 ACL 另拒非可信主体修改元数据/ACL/owner。
    // 共享锁只保护 rename/delete，不能阻止 FILE_WRITE_ATTRIBUTES reparse 控制。
    let mut current = std::path::PathBuf::from(format!("{}:\\", drive as char));
    held.push(directory(&current)?);
    for component in &components[2..] {
        current.push(component.as_os_str());
        let file = directory(&current)?;
        super::acl::ancestor(&file)?;
        held.push(file);
    }
    Ok(held)
}
pub(super) fn directory(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    check(&file, true)?;
    Ok(file)
}
pub(super) fn binary(path: &Path) -> io::Result<File> {
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
