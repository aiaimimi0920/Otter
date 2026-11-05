//! 显式受管图片目录的直接子文件准入；固定目录句柄，文件按最终句柄路径再次核验。
#[path = "session_asset_archive.rs"]
pub mod archive;
#[path = "session_asset_publish.rs"]
pub mod publish;
use std::{
    fs::{File, OpenOptions},
    os::windows::{
        ffi::OsStringExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_ACCESSDENIED, E_FAIL, E_INVALIDARG, HANDLE},
        Storage::FileSystem::*,
    },
};

pub struct Root {
    _directory: File,
    requested: String,
    resolved: PathBuf,
}
impl Root {
    /// root 必须由应用配置/明确选择提供，不能从待恢复会话自身推导授权目录。
    pub fn open(root: &Path) -> Result<Self> {
        let requested = normalize(root.to_str().ok_or_else(denied)?)?;
        let directory = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
            .open(root)
            .map_err(io_error)?;
        if !directory.metadata().map_err(io_error)?.is_dir() {
            return Err(denied());
        }
        let resolved = final_path(&directory)?;
        Ok(Self {
            _directory: directory,
            requested,
            resolved,
        })
    }
    pub fn open_image(&self, reference: &str) -> Result<File> {
        let candidate = normalize(reference)?;
        let prefix = format!("{}\\", self.requested.trim_end_matches('\\'));
        let head = candidate.get(..prefix.len()).ok_or_else(denied)?;
        if !head.eq_ignore_ascii_case(&prefix) {
            return Err(denied());
        }
        let name = &candidate[prefix.len()..];
        if !valid_name(name) {
            return Err(denied());
        }
        // 从固定的实际目录路径打开，仅共享读；禁止跟随最终文件的重解析点。
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(self.resolved.join(name))
            .map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(denied());
        }
        let actual = final_path(&file)?;
        if !actual.parent().is_some_and(|parent| {
            parent
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&self.resolved.as_os_str().to_string_lossy())
        }) {
            return Err(denied());
        }
        Ok(file)
    }
}
fn denied() -> Error {
    Error::new(
        E_ACCESSDENIED,
        "Session image path is outside its admitted asset directory",
    )
}
fn io_error(error: std::io::Error) -> Error {
    // 保留系统错误身份，使发布 owner 能区分短暂共享冲突与永久拒绝；不解析本地化消息。
    let code = error.raw_os_error().map_or(E_FAIL, |code| {
        windows::core::HRESULT::from_win32(code as u32)
    });
    Error::new(code, format!("Session asset open failed: {error}"))
}
fn normalize(value: &str) -> Result<String> {
    let value = value.replace('/', "\\");
    let value = value.strip_prefix("\\\\?\\").unwrap_or(&value);
    let bytes = value.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || value.len() > 32768
        || value.chars().any(char::is_control)
    {
        return Err(denied());
    }
    Ok(value.trim_end_matches('\\').into())
}
fn valid_name(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let numbered_device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    !matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) && !numbered_device
}
fn final_path(file: &File) -> Result<PathBuf> {
    let mut path = vec![0u16; 32768];
    let length = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(file.as_raw_handle()),
            &mut path,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    } as usize;
    if length == 0 {
        return Err(Error::from_win32());
    }
    if length >= path.len() {
        return Err(Error::new(
            E_INVALIDARG,
            "Final session asset path exceeds budget",
        ));
    }
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &path[..length],
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_namespaces_do_not_admit_devices_or_nested_paths() {
        for name in [
            "..",
            "../a.png",
            "a\\b.png",
            "a.png:stream",
            "CON.png",
            "LPT1",
            "bad.",
            "bad ",
        ] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("中文_source_0123456789abcdef.png"));
        for path in [
            "https://host/a",
            "\\\\host\\share\\a",
            "\\\\.\\pipe\\a",
            "relative.png",
            "C:relative",
        ] {
            assert!(normalize(path).is_err());
        }
    }
    #[test]
    fn real_handle_stays_in_explicit_root_and_outside_reference_is_rejected() {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-assets-{}",
            super::super::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("image.png");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let admitted = Root::open(&root).unwrap();
        let file = admitted.open_image(path.to_str().unwrap()).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 3);
        assert!(admitted
            .open_image(root.join("..\\outside.png").to_str().unwrap())
            .is_err());
        assert!(admitted.open_image("C:\\outside.png").is_err());
        assert!(admitted
            .open_image(root.join("nested\\image.png").to_str().unwrap())
            .is_err());
        drop(file);
        drop(admitted);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
