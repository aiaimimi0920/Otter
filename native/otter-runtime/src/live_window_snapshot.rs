//! External-source recovery snapshot; reads do not change windows or imply hiding authorization/crash-recovery registration.
#[path = "live_window_hide.rs"]
pub mod hide;
#[path = "live_window_restore.rs"]
pub mod restore;
use super::Target;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy, Debug)]
pub struct Layered {
    pub color: COLORREF,
    pub alpha: u8,
    pub flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS,
}

/// Retain original values instead of inferring them again from the current window; later restoration must revalidate target.
pub struct Snapshot {
    pub target: Target,
    pub rect: RECT,
    pub placement: WINDOWPLACEMENT,
    pub iconic: bool,
    pub visible: bool,
    pub extended_style: isize,
    pub layered: Option<Layered>,
}
impl Snapshot {
    pub fn read(target: &Target) -> Result<Self> {
        target.validate()?;
        unsafe {
            let hwnd = target.hwnd();
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            let mut placement = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            GetWindowPlacement(hwnd, &mut placement)?;
            let iconic = IsIconic(hwnd).as_bool();
            let visible = IsWindowVisible(hwnd).as_bool();
            SetLastError(ERROR_SUCCESS);
            let extended_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if extended_style == 0 && GetLastError() != ERROR_SUCCESS {
                return Err(Error::from_win32());
            }
            let layered = if extended_style & WS_EX_LAYERED.0 as isize != 0 {
                let mut attributes = Layered {
                    color: COLORREF::default(),
                    alpha: 0,
                    flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS::default(),
                };
                // Paths such as UpdateLayeredWindow may not permit reading this attribute. Reject failures rather than guessing defaults.
                GetLayeredWindowAttributes(
                    hwnd,
                    Some(&mut attributes.color),
                    Some(&mut attributes.alpha),
                    Some(&mut attributes.flags),
                )?;
                Some(attributes)
            } else {
                None
            };
            target.validate()?;
            Ok(Self {
                target: target.clone(),
                rect,
                placement,
                iconic,
                visible,
                extended_style,
                layered,
            })
        }
    }
}

#[cfg(test)]
#[path = "live_window_snapshot_tests.rs"]
mod tests;
