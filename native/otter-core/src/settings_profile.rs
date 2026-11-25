//! 原生配置路径选择；与主干目录隔离，纯函数不探测/创建文件。
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
pub struct Selection {
    pub path: PathBuf,
    pub allow_missing: bool,
}
pub fn select(
    explicit: Option<&Path>,
    override_root: Option<&OsStr>,
    local_app_data: Option<&OsStr>,
    hidden_check: bool,
) -> Result<Option<Selection>, &'static str> {
    if let Some(path) = explicit {
        if !path.is_absolute() {
            return Err("Explicit settings path must be absolute");
        }
        return Ok(Some(Selection {
            path: path.to_path_buf(),
            allow_missing: false,
        }));
    }
    if hidden_check && override_root.is_none() {
        return Ok(None);
    }
    let root = if let Some(root) = override_root {
        PathBuf::from(root)
    } else {
        PathBuf::from(local_app_data.ok_or("LOCALAPPDATA is required for the native profile")?)
            .join("OtterNative")
    };
    if !root.is_absolute() {
        return Err("Native profile directory must be absolute");
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(root.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(_, _)))
        {
            return Err("Native profile requires an ordinary filesystem path");
        }
    }
    Ok(Some(Selection {
        path: root.join("app-settings.json"),
        allow_missing: true,
    }))
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn explicit_path_wins_and_native_default_is_isolated() {
        let explicit = Path::new("C:/chosen.json");
        let selected = select(Some(explicit), Some(OsStr::new("invalid")), None, false)
            .unwrap()
            .unwrap();
        assert_eq!(selected.path, explicit);
        assert!(!selected.allow_missing);
        let selected = select(None, None, Some(OsStr::new("C:/User/Local")), false)
            .unwrap()
            .unwrap();
        assert_eq!(
            selected.path,
            Path::new("C:/User/Local/OtterNative/app-settings.json")
        );
        assert!(selected.allow_missing);
    }
    #[test]
    fn hidden_checks_do_not_discover_real_user_profile_and_overrides_are_explicit() {
        assert!(select(None, None, Some(OsStr::new("C:/real")), true)
            .unwrap()
            .is_none());
        let selected = select(None, Some(OsStr::new("C:/isolated")), None, true)
            .unwrap()
            .unwrap();
        assert_eq!(selected.path, Path::new("C:/isolated/app-settings.json"));
        assert!(select(None, Some(OsStr::new("relative")), None, false).is_err());
        assert!(select(None, Some(OsStr::new("")), None, false).is_err());
        assert!(select(None, None, None, false).is_err());
    }
}
