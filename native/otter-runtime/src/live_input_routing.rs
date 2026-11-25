//! Read-only hit/focus routing within the source window; no focus stealing, transparent-style changes, or desktop-window enumeration.
use super::{
    input_coordinates::Dpi,
    input_target::{self, Destination},
    Target,
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
const MAX_DEPTH: usize = 64;
#[path = "live_input_caret.rs"]
pub mod caret;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Target(input_target::Error),
    Transform,
    DepthLimit,
}
type Result<T> = std::result::Result<T, Error>;

/// root_client must come from source-DPI coordinate conversion. Descend only within the specified root, without WindowFromPoint.
pub fn pointer(root: &Target, root_client: [i32; 2]) -> Result<Destination> {
    root.validate()
        .map_err(|_| Error::Target(input_target::Error::SourceChanged))?;
    let _dpi = Dpi::enter(root.hwnd()).map_err(|_| Error::Transform)?;
    let mut screen = POINT {
        x: root_client[0],
        y: root_client[1],
    };
    if !unsafe { ClientToScreen(root.hwnd(), &mut screen) }.as_bool() {
        return Err(Error::Transform);
    }
    let mut current = root.hwnd();
    let mut remaining = 256;
    for _ in 0..MAX_DEPTH {
        let Some(child) = child_at(current, screen, &mut remaining)? else {
            return nearest(root, current);
        };
        if !unsafe { IsChild(root.hwnd(), child) }.as_bool() {
            return Err(Error::Target(input_target::Error::NotDescendant));
        }
        current = child;
    }
    Err(Error::DepthLimit)
}
/// Traverse direct children in Z order. Transparent roots neither depend on empty system hit-test results nor change styles.
/// Check at most 256 child windows per search; reject abnormal trees instead of traversing indefinitely.
fn child_at(parent: HWND, screen: POINT, remaining: &mut usize) -> Result<Option<HWND>> {
    if !unsafe { IsWindowVisible(parent) }.as_bool() {
        return Ok(None);
    }
    let mut local = screen;
    let mut client = RECT::default();
    if !unsafe { ScreenToClient(parent, &mut local) }.as_bool() {
        return Err(Error::Transform);
    }
    unsafe { GetClientRect(parent, &mut client) }.map_err(|_| Error::Transform)?;
    if !contains(client, local) {
        return Ok(None);
    }
    let mut next = unsafe { GetWindow(parent, GW_CHILD) }.ok();
    while let Some(child) = next {
        if *remaining == 0 {
            return Err(Error::DepthLimit);
        }
        *remaining -= 1;
        if unsafe { GetParent(child) }.ok() != Some(parent) {
            return Err(Error::Transform);
        }
        let visible = unsafe { IsWindowVisible(child) }.as_bool();
        let enabled = unsafe { IsWindowEnabled(child) }.as_bool();
        let transparent =
            unsafe { GetWindowLongPtrW(child, GWL_EXSTYLE) } & WS_EX_TRANSPARENT.0 as isize != 0;
        if visible && enabled && !transparent {
            let mut bounds = RECT::default();
            unsafe { GetWindowRect(child, &mut bounds) }.map_err(|_| Error::Transform)?;
            if contains(bounds, screen) {
                return Ok(Some(child));
            }
        }
        next = unsafe { GetWindow(child, GW_HWNDNEXT) }.ok();
    }
    Ok(None)
}
fn contains(rect: RECT, point: POINT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}
/// Do not call SetFocus/AttachThreadInput; if another window has focus, fall back only to the original root.
pub fn keyboard(root: &Target) -> Result<Destination> {
    keyboard_with_fallback(root, None)
}
pub fn keyboard_with_fallback(
    root: &Target,
    fallback: Option<&Destination>,
) -> Result<Destination> {
    root.validate()
        .map_err(|_| Error::Target(input_target::Error::SourceChanged))?;
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let focus = if unsafe { GetGUIThreadInfo(root.identity.thread, &mut info) }.is_ok() {
        info.hwndFocus
    } else {
        HWND::default()
    };
    keyboard_at(root, focus, fallback)
}
// Accept focus only within this root/child; when the source queue has no focus, retain the validated click target.
fn keyboard_at(root: &Target, focus: HWND, fallback: Option<&Destination>) -> Result<Destination> {
    if focus == root.hwnd() || unsafe { IsChild(root.hwnd(), focus) }.as_bool() {
        nearest(root, focus)
    } else if let Some(fallback) = fallback {
        retained(root, fallback)
    } else {
        nearest(root, root.hwnd())
    }
}
/// Existing drags/keys preserve their target first; invalidation falls back only to the authorized root, never hit-testing another control.
pub fn retained(root: &Target, previous: &Destination) -> Result<Destination> {
    if !root.same_source(previous.root()) {
        return Err(Error::Target(input_target::Error::NotDescendant));
    }
    if previous.preflight().is_ok() {
        return Ok(previous.clone());
    }
    Destination::bind(root, root.hwnd()).map_err(Error::Target)
}
fn nearest(root: &Target, mut candidate: HWND) -> Result<Destination> {
    for _ in 0..MAX_DEPTH {
        match Destination::bind(root, candidate) {
            Ok(target) => return Ok(target),
            Err(error) if candidate == root.hwnd() => return Err(Error::Target(error)),
            Err(_) => {}
        }
        let parent = unsafe { GetParent(candidate) }.ok();
        candidate = match parent {
            Some(parent)
                if parent == root.hwnd() || unsafe { IsChild(root.hwnd(), parent) }.as_bool() =>
            {
                parent
            }
            _ => root.hwnd(),
        };
    }
    Err(Error::DepthLimit)
}
#[cfg(test)]
#[path = "live_input_routing_tests.rs"]
mod tests;
