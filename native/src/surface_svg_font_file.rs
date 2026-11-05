//! 系统字体文件租约：校验最终句柄后读取同一句柄，不能先校验路径再重新打开。
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HANDLE,
        Globalization::{CompareStringOrdinal, CSTR_EQUAL},
        Storage::FileSystem::*,
        System::SystemInformation::GetWindowsDirectoryW,
    },
};

pub(super) const MAX_FILE: usize = 24 * 1024 * 1024;
pub(super) fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Untrusted or over-budget SVG font",
    )
}
pub(super) struct Root {
    _directory: File,
    path: PathBuf,
}
impl Root {
    pub(super) fn system() -> io::Result<Self> {
        let mut buffer = vec![0u16; 32768];
        let n = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if n == 0 || n >= buffer.len() {
            return Err(invalid());
        }
        Self::open(&PathBuf::from(OsString::from_wide(&buffer[..n])).join("Fonts"))
    }
    fn open(path: &Path) -> io::Result<Self> {
        let directory = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?;
        let metadata = directory.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(invalid());
        }
        let path = final_path(&directory)?;
        Ok(Self {
            _directory: directory,
            path,
        })
    }
    pub(super) fn read(&self, path: &Path, remaining: usize) -> io::Result<Vec<u8>> {
        let mut file = self.admit(path)?;
        let length = usize::try_from(file.metadata()?.len()).map_err(|_| invalid())?;
        if length == 0 || length > MAX_FILE || length > remaining {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| invalid())?;
        bytes.resize(length, 0);
        file.read_exact(&mut bytes)?;
        if file.read(&mut [0u8; 1])? != 0 {
            return Err(invalid());
        }
        Ok(bytes)
    }
    fn admit(&self, path: &Path) -> io::Result<File> {
        let normalized: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .map(|c| if c == 47 { 92 } else { c })
            .collect();
        let normalized = PathBuf::from(OsString::from_wide(&normalized));
        let path = normalized.as_path();
        // 先排除设备/UNC/ADS 名称，避免对非文件对象执行可能阻塞的打开。
        if !plain_local_path(path)
            || !same_parent(path, &self.path)
            || !safe_name(path)
            || !path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
                ["ttf", "ttc", "otf"]
                    .iter()
                    .any(|ext| s.eq_ignore_ascii_case(ext))
            })
        {
            return Err(invalid());
        }
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
            || unsafe { GetFileType(HANDLE(file.as_raw_handle())) } != FILE_TYPE_DISK
        {
            return Err(invalid());
        }
        let actual = final_path(&file)?;
        if !same_parent(&actual, &self.path) {
            return Err(invalid());
        }
        Ok(file)
    }
}
fn safe_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        && !["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|s| {
                matches!(
                    s,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
}
fn same_parent(path: &Path, root: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    let a: Vec<u16> = parent.as_os_str().encode_wide().collect();
    let b: Vec<u16> = root.as_os_str().encode_wide().collect();
    unsafe { CompareStringOrdinal(&a, &b, true) == CSTR_EQUAL }
}
fn plain_local_path(path: &Path) -> bool {
    let s: Vec<u16> = path.as_os_str().encode_wide().collect();
    s.len() >= 4
        && s.len() <= 32767
        && (s[0] as u8).is_ascii_alphabetic()
        && s[0] < 128
        && s[1] == 58
        && s[2] == 92
        && !s[2..].iter().any(|v| *v == 0 || *v == 58)
}
fn final_path(file: &File) -> io::Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let n = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(file.as_raw_handle()),
            &mut buffer,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    } as usize;
    if n == 0 || n >= buffer.len() {
        return Err(invalid());
    }
    let prefix: Vec<u16> = "\\\\?\\".encode_utf16().collect();
    if !buffer[..n].starts_with(&prefix) {
        return Err(invalid());
    }
    let path = PathBuf::from(OsString::from_wide(&buffer[4..n]));
    if !plain_local_path(&path) {
        return Err(invalid());
    }
    let drive = [buffer[4], 58, 92, 0];
    // DRIVE_FIXED=3；磁盘类型检查本身不能替代下面的最终目录归属比较。
    if unsafe { GetDriveTypeW(PCWSTR(drive.as_ptr())) } != 3 {
        return Err(invalid());
    }
    Ok(path)
}

#[cfg(test)]
#[path = "surface_svg_font_file_tests.rs"]
mod tests;
