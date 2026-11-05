//! 默认 PNG 保存：目录与主干隔离；只在显式命令时冻结快照，目录创建/编码/分配在 worker。
use super::{export, State};
use std::path::{Path, PathBuf};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindow};

pub(in crate::desktop) unsafe fn start(hwnd: HWND, state: &State) {
    if state.modals.blocked() || state.export.borrow().is_some() || !IsWindow(Some(hwnd)).as_bool()
    {
        return;
    }
    let override_root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    // --settings 只指定配置文件，不把任意导入文件的父目录授权为默认图片输出位置。
    let root =
        crate::settings_profile::select(None, override_root.as_deref(), local.as_deref(), false)
            .and_then(|selection| {
                selection
                    .and_then(|value| value.path.parent().map(Path::to_path_buf))
                    .ok_or("Missing native profile")
            });
    let root = match root {
        Ok(root) => root,
        Err(error) => {
            super::report(hwnd, error);
            return;
        }
    };
    let prepared = match prepare(state, root) {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return,
        Err(error) => {
            super::report(hwnd, &error);
            return;
        }
    };
    super::spawn(hwnd, state, move || prepared.save().map(|_| ()));
}

struct Prepared {
    root: PathBuf,
    stem: String,
    image: crate::desktop::image::Image,
    editor: crate::editor::Editor,
    opacity: f32,
}
fn prepare(state: &State, root: PathBuf) -> Result<Option<Prepared>, String> {
    if state.modals.blocked() || state.export.borrow().is_some() {
        return Ok(None);
    }
    state.canvas.borrow_mut().cancel_gesture();
    let snapshot = crate::desktop::display_snapshot::capture(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
    );
    let Some((image, editor)) = snapshot else {
        return Ok(None);
    };
    let name =
        crate::desktop::export_naming::suggested(state).ok_or("Cannot render PNG filename")?;
    let stem = name
        .strip_suffix(".png")
        .ok_or("Invalid PNG filename")?
        .to_owned();
    let opacity = state.canvas.borrow().normal_opacity;
    Ok(Some(Prepared {
        root,
        stem,
        image,
        editor,
        opacity,
    }))
}
impl Prepared {
    fn save(self) -> Result<PathBuf, String> {
        if !self.root.is_absolute() {
            return Err("Default save directory must be absolute".into());
        }
        let directory = self.root.join("saved");
        // 不接受已存在的最终重解析目录；这不是对恶意祖先路径竞态的完整沙箱。
        for path in [&self.root, &directory] {
            crate::profile_directory::validate(path).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        for path in [&self.root, &directory] {
            crate::profile_directory::validate(path).map_err(|e| e.to_string())?;
        }
        export::save_incremented(
            &directory,
            &self.stem,
            &self.image,
            &self.editor,
            self.opacity,
        )
    }
}

#[cfg(test)]
#[path = "export_default_tests.rs"]
mod tests;
