//! Default PNG saving isolates directories from the baseline; freeze snapshots only on explicit commands, with directory creation/encoding/allocation in workers.
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
    // --settings selects configuration only; arbitrary imported-file parent directories are not authorized as default image-output destinations.
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
    style: export::style::Snapshot,
}
fn prepare(state: &State, root: PathBuf) -> Result<Option<Prepared>, String> {
    if state.modals.blocked() || state.export.borrow().is_some() {
        return Ok(None);
    }
    if !crate::desktop::display_snapshot::ready(state) {
        return Err(crate::ui_text::text("display-output-edit-pending").into());
    }
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
        style: export::style::Snapshot::capture_with_frame(
            &state.canvas.borrow(),
            state.resolved_display.borrow().as_ref(),
        ),
    }))
}
impl Prepared {
    fn save(self) -> Result<PathBuf, String> {
        if !self.root.is_absolute() {
            return Err("Default save directory must be absolute".into());
        }
        let directory = self.root.join("saved");
        // Reject existing final reparse-point directories; this is not a complete sandbox against malicious ancestor-path races.
        for path in [&self.root, &directory] {
            crate::profile_directory::validate(path).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        for path in [&self.root, &directory] {
            crate::profile_directory::validate(path).map_err(|e| e.to_string())?;
        }
        self.style.save_incremented(
            &directory,
            &self.stem,
            &self.image,
            &self.editor,
            self.opacity,
        )
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "export_default_tests.rs"]
mod tests;
