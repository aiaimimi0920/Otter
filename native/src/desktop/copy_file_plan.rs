//! 显式复制时冻结命名与隔离缓存目录；不读写磁盘，worker 才暂存已编码 PNG。
use super::super::State;
use std::{path::PathBuf, sync::atomic::AtomicBool};
pub(in crate::desktop) struct Plan {
    pub root: PathBuf,
    pub stem: String,
    pub policy: crate::cache_policy::Policy,
}
impl Plan {
    pub(super) fn from_state(state: &State) -> Result<Self, String> {
        let filename = crate::desktop::export_naming::suggested_clipboard(state)
            .ok_or("Cannot render clipboard filename")?;
        let stem = filename
            .strip_suffix(".png")
            .ok_or("Invalid clipboard filename")?
            .to_owned();
        let root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
        let local = std::env::var_os("LOCALAPPDATA");
        let selected =
            crate::settings_profile::select(None, root.as_deref(), local.as_deref(), false)
                .map_err(str::to_owned)?
                .ok_or("Missing native clipboard profile")?;
        Ok(Self {
            root: selected
                .path
                .parent()
                .ok_or("Missing native clipboard directory")?
                .to_path_buf(),
            stem,
            policy: state.cache_policy.get(),
        })
    }
    pub(super) fn stage(
        self,
        png: crate::pixels::Pixels,
        cancel: &AtomicBool,
    ) -> Result<crate::clipboard_cache::Prepared, String> {
        crate::clipboard_cache::stage_with_policy(&self.root, &self.stem, png, cancel, self.policy)
            .map_err(|e| e.to_string())
    }
}
