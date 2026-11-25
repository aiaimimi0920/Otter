//! Number paste transaction: pass the same snapshot to EM_REPLACESEL; invalidate if delayed-rendering reentrancy changes the target.
#[path = "surface_number_clipboard.rs"]
mod clipboard;
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        UI::{Controls::*, Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
    },
};
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Number.Paste.Owner");
struct Guard {
    hwnd: HWND,
    marker: Box<u8>,
}
impl Guard {
    unsafe fn new(hwnd: HWND) -> Option<Self> {
        if !GetPropW(hwnd, OWNER).is_invalid() {
            return None;
        }
        let guard = Self {
            hwnd,
            marker: Box::new(0),
        };
        SetPropW(
            hwnd,
            OWNER,
            Some(HANDLE((&*guard.marker as *const u8).cast_mut().cast())),
        )
        .ok()?;
        Some(guard)
    }
    unsafe fn current(&self) -> bool {
        GetPropW(self.hwnd, OWNER).0 == (&*self.marker as *const u8).cast_mut().cast()
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            if self.current() {
                let _ = RemovePropW(self.hwnd, OWNER);
            }
        }
    }
}
pub unsafe fn invalidate(hwnd: HWND) {
    let _ = RemovePropW(hwnd, OWNER);
}
unsafe fn eligible(hwnd: HWND) -> bool {
    IsWindowEnabled(hwnd).as_bool()
        && GetWindowLongW(hwnd, GWL_STYLE) as u32 & ES_READONLY as u32 == 0
        && GetParent(hwnd).is_ok_and(|parent| {
            SendMessageW(
                parent,
                super::CAN_PASTE,
                None,
                Some(LPARAM(hwnd.0 as isize)),
            )
            .0 != 0
        })
}
pub unsafe fn paste(hwnd: HWND) {
    paste_with(hwnd, || clipboard::read(hwnd));
}
pub(crate) unsafe fn paste_with(hwnd: HWND, read: impl FnOnce() -> Option<Vec<u16>>) {
    if !eligible(hwnd) {
        return;
    }
    let Some(guard) = Guard::new(hwnd) else {
        return;
    };
    let Some(mut text) = read() else {
        return;
    };
    // GetClipboardData may trigger external delayed rendering; edits, selection, focus, permission/composition changes then
    // clear the temporary marker. Box remains alive, so even a reused old HWND cannot impersonate this operation.
    if !guard.current()
        || !eligible(hwnd)
        || text.len() > super::super::surface_edit_control::MAX_UNITS
        || text.contains(&0)
    {
        return;
    }
    text.push(0);
    // Eligibility queries currently have no side effects; still recheck marker to avoid stale conclusions if future wiring adds reentrancy.
    if !guard.current() {
        return;
    }
    // The clipboard is closed when read returns; use only that copied snapshot without sending WM_PASTE again.
    SendMessageW(
        hwnd,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(text.as_ptr() as isize)),
    );
}
