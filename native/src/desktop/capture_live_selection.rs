//! Live 的冻结前身份快照与选区交付；不按标题重查来源，不在窗口失败后静默捕获桌面。
use super::Request;
use crate::live_window_target::{region::Crop, Target};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::GetCurrentProcessId, UI::WindowsAndMessaging::*},
};

struct Candidate {
    target: Target,
    bounds: RECT,
}
fn ignored_class(class: &str) -> bool {
    matches!(
        class,
        "Progman"
            | "WorkerW"
            | "SHELLDLL_DefView"
            | "SysListView32"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "DV2ControlHost"
            | "Tauri Window"
            | "Tooltip"
            | "ToolTips_Class32"
    )
}
fn snapshot() -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    for (hwnd, _) in super::super::super::capture_windows::entries()? {
        let mut process = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut process));
            if process == 0
                || process == GetCurrentProcessId()
                || hwnd == GetShellWindow()
                || hwnd == GetDesktopWindow()
            {
                continue;
            }
            let mut name = [0u16; 256];
            let length = GetClassNameW(hwnd, &mut name);
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if length <= 0
                || ignored_class(&String::from_utf16_lossy(&name[..length as usize]))
                || style & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0)
                    == WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0
            {
                continue;
            }
        }
        let Ok(target) = Target::bind(hwnd) else {
            continue;
        };
        let Ok(bounds) = target.frame_bounds() else {
            continue;
        };
        candidates.push(Candidate { target, bounds });
    }
    Ok(candidates)
}
fn contains(outer: RECT, inner: RECT) -> bool {
    inner.right > inner.left
        && inner.bottom > inner.top
        && inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}
fn resolve(candidates: Vec<Candidate>, screen: RECT, whole: bool) -> Result<Request> {
    let selected = candidates
        .into_iter()
        .find(|candidate| contains(candidate.bounds, screen));
    let Some(candidate) = selected else {
        if whole {
            return Err(Error::new(E_ABORT, "Selected Live window unavailable"));
        }
        crate::capture_target::Target::resolve(screen)
            .map_err(|error| Error::new(E_FAIL, format!("Invalid Live target: {error:?}")))?;
        return Ok(Request::Region(screen));
    };
    // 选区期间移动/resize/关闭不重新解释同一屏幕 RECT；用户应重新选择。
    if candidate.target.frame_bounds()? != candidate.bounds {
        return Err(Error::new(E_ABORT, "Live source moved during selection"));
    }
    if whole || candidate.bounds == screen {
        return Ok(Request::Window(candidate.target));
    }
    let crop = Crop::from_screen(candidate.bounds, screen)?;
    Ok(Request::WindowRegion(candidate.target, crop))
}
pub(super) fn select() -> Result<Option<Request>> {
    let candidates = snapshot()?;
    let rectangles = candidates
        .iter()
        .map(|candidate| candidate.bounds)
        .collect();
    let frame = super::super::super::capture::virtual_desktop()?;
    let selected =
        super::super::super::capture_region::select_live(frame.image, frame.origin, rectangles)?;
    selected
        .map(|(screen, whole)| resolve(candidates, screen, whole))
        .transpose()
}

#[cfg(test)]
#[path = "capture_live_selection_tests.rs"]
mod tests;
