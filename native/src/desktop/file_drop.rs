//! Shell file drops create new pins; copy bounded paths and release HDROP before starting background decoding.
use super::State;
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};

struct DropFiles(HDROP);
impl Drop for DropFiles {
    fn drop(&mut self) {
        unsafe {
            DragFinish(self.0);
        }
    }
}

unsafe fn paths(drop: DropFiles) -> Result<Vec<PathBuf>> {
    let count = DragQueryFileW(drop.0, u32::MAX, None);
    if count == 0 || count as usize > super::open_job::MAX_BATCH {
        return Err(Error::new(
            E_INVALIDARG,
            crate::ui_text::text("action-drop-count"),
        ));
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let length = DragQueryFileW(drop.0, index, None) as usize;
        if length == 0 || length > 32767 {
            return Err(Error::new(
                E_INVALIDARG,
                crate::ui_text::text("action-drop-path"),
            ));
        }
        let mut name = vec![0u16; length + 1];
        if DragQueryFileW(drop.0, index, Some(&mut name)) as usize != length
            || name[length] != 0
            || name[..length].contains(&0)
        {
            return Err(Error::new(
                E_INVALIDARG,
                crate::ui_text::text("action-drop-incomplete"),
            ));
        }
        paths.push(PathBuf::from(OsString::from_wide(&name[..length])));
    }
    Ok(paths)
}

pub(super) unsafe fn receive(hwnd: HWND, state: &State, parameter: WPARAM) {
    // Take and release Shell packets even when the entry point is busy; never leak reentrantly delivered HDROP.
    let dropped = DropFiles(HDROP(parameter.0 as *mut _));
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let result = paths(dropped).and_then(|paths| open(hwnd, state, paths));
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(crate::ui_text::format(
                    "action-import-failed",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

unsafe fn open(hwnd: HWND, state: &State, paths: Vec<PathBuf>) -> Result<()> {
    if state.open_job.borrow().is_some() {
        return Err(Error::new(
            E_FAIL,
            crate::ui_text::text("action-image-loading"),
        ));
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
    // Do not probe/decode files on UI threads; WIC failures preserve original documents.
    super::open_job::start_destination(hwnd, state, super::open_job::Input::Files(paths), true)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "file_drop_tests.rs"]
mod tests;
