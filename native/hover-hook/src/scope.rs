//! HWND properties are short-lived bindings, not authentication credentials; hosts still perform original input-permission checks first.
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};
pub const COOKIE: PCWSTR = w!("Otter.Live.Hover.Cookie.v1");
pub const ACTIVE: PCWSTR = w!("Otter.Live.Hover.Active.v1");
pub const SOURCE: PCWSTR = w!("Otter.Live.Hover.Source.v1");
pub const OWNER: PCWSTR = w!("Otter.Live.Hover.Owner.v1");
pub const SOURCE_COOKIE: PCWSTR = w!("Otter.Live.Hover.SourceCookie.v1");

/// # Safety
/// Query only HWND, never dereference window-property values. Call on source message threads.
pub unsafe fn suppress(message_window: HWND) -> bool {
    let root = GetAncestor(message_window, GA_ROOT);
    let mut process = 0;
    let thread = GetWindowThreadProcessId(root, Some(&mut process));
    if process != GetCurrentProcessId()
        || thread != GetCurrentThreadId()
        || IsIconic(root).as_bool()
    {
        return false;
    }
    let owner = HWND(GetPropW(root, OWNER).0);
    let cookie = GetPropW(root, SOURCE_COOKIE);
    if cookie.is_invalid()
        || !IsWindow(Some(owner)).as_bool()
        || !IsWindowVisible(owner).as_bool()
        || GetPropW(owner, COOKIE) != cookie
        || GetPropW(owner, ACTIVE) != cookie
        || GetPropW(owner, SOURCE).0 != root.0
    {
        return false;
    }
    let mut cursor = POINT::default();
    GetCursorPos(&mut cursor).is_ok() && GetAncestor(WindowFromPoint(cursor), GA_ROOT) == owner
        // Revocation may overlap the cross-process query above; recheck complete bindings finally.
        && GetPropW(owner, COOKIE) == cookie && GetPropW(owner, ACTIVE) == cookie
        && GetPropW(root, OWNER).0 == owner.0 && GetPropW(root, SOURCE_COOKIE) == cookie
}
