//! 显式取消的持久开关；仅在连接/取消边界读写一个有界小文件，不触碰设备身份。
use std::{path::Path, sync::atomic::AtomicBool};
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preference {
    pub paused: bool,
    pub origin: String,
}
pub(super) fn load(root: &Path) -> Result<Option<Preference>, String> {
    match read(root)? {
        None => Ok(None),
        Some(bytes) => {
            let value: Preference =
                serde_json::from_slice(&bytes).map_err(|_| "Loom 自动连接设置无效")?;
            if crate::device_session_attempt::canonical_origin(&value.origin).as_ref()
                != Ok(&value.origin)
            {
                return Err("Loom 自动连接地址无效".into());
            }
            Ok(Some(value))
        }
    }
}
fn read(root: &Path) -> Result<Option<Vec<u8>>, String> {
    crate::profile_directory::validate(root).map_err(|e| e.to_string())?;
    match crate::desktop::session_job::file::read_limited(
        &root.join("loom-autoconnect-paused.json"),
        &AtomicBool::new(false),
        4096,
    ) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
pub(super) fn save(root: &Path, paused: bool, origin: &str) -> Result<(), String> {
    let _pins = crate::profile_directory::pin(root).map_err(|e| e.to_string())?;
    let path = root.join("loom-autoconnect-paused.json");
    let cancel = AtomicBool::new(false);
    let next = serde_json::to_vec(&Preference {
        paused,
        origin: origin.into(),
    })
    .map_err(|e| e.to_string())?;
    let lease =
        crate::pixels::reserve(next.capacity()).ok_or("Loom preference byte budget exhausted")?;
    let next = crate::pixels::Pixels::from_reserved(next, lease);
    if let Some(before) = read(root)? {
        if before.as_slice() == &*next {
            return Ok(());
        }
        crate::session_file_replace::Replacement::prepare_bytes(&path, &before, next, &cancel)
            .map_err(|e| e.to_string())?
            .commit(&cancel)
            .map_err(|_| "Loom 自动连接设置保存失败".to_owned())?;
    } else {
        crate::session_file_write::Staged::prepare_bytes(&path, next, &cancel)
            .map_err(|e| e.to_string())?
            .commit_new(&cancel)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_loom_preference_survives_reopen_and_rejects_corrupt_state() {
        let root = std::env::temp_dir().join(format!(
            "otter-loom-preference-{:?}",
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
        ));
        assert!(load(&root).unwrap().is_none());
        save(&root, true, "https://192.168.1.20:8766").unwrap();
        let saved = load(&root).unwrap().unwrap();
        assert!(saved.paused);
        assert_eq!(saved.origin, "https://192.168.1.20:8766");
        save(&root, false, &saved.origin).unwrap();
        assert!(!load(&root).unwrap().unwrap().paused);
        std::fs::write(root.join("loom-autoconnect-paused.json"), b"invalid").unwrap();
        assert!(load(&root).is_err());
    }
}
