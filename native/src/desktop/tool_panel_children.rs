//! Create panel child buttons directly from current DIP/scroll geometry; layout positions only actual HWNDs whose geometry changed.
//! Temporary Window collections own them until successful publication, rolling back failures; afterward the parent panel owns them without geometry caching.
use super::{entries, geometry, scroll, Window};
use windows::{
    core::{w, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::ScreenToClient,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub(super) unsafe fn create(panel: HWND, instance: HINSTANCE) -> Result<Vec<Window>> {
    create_checked(panel, instance, |_| Ok(()))
}

pub(super) unsafe fn create_checked(
    panel: HWND,
    instance: HINSTANCE,
    mut before: impl FnMut(usize) -> Result<()>,
) -> Result<Vec<Window>> {
    let dpi = GetDpiForWindow(panel);
    // Ranges derive from fixed catalogs/client areas, independent of button existence; final layout still handles DPI/size changes.
    let offset = scroll::sync(panel);
    let mut controls = Vec::with_capacity(entries().count());
    for (index, (id, label)) in entries().enumerate() {
        before(index)?;
        let rect = geometry::button(index, crate::desktop::tool_commands::TOOLS.len(), dpi);
        controls.push(Window(CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            &HSTRING::from(label.replace('\t', " ")),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            rect.left - offset.x,
            rect.top - offset.y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(panel),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )?));
    }
    Ok(controls)
}

unsafe fn matches(button: HWND, parent: HWND, expected: RECT, offset: POINT) -> bool {
    let mut actual = RECT::default();
    if GetWindowRect(button, &mut actual).is_err() {
        return false;
    }
    let mut origin = POINT {
        x: actual.left,
        y: actual.top,
    };
    ScreenToClient(parent, &mut origin).as_bool()
        && origin.x == expected.left - offset.x
        && origin.y == expected.top - offset.y
        && i64::from(actual.right) - i64::from(actual.left)
            == i64::from(expected.right - expected.left)
        && i64::from(actual.bottom) - i64::from(actual.top)
            == i64::from(expected.bottom - expected.top)
}

pub(super) unsafe fn layout(panel: HWND) {
    let dpi = GetDpiForWindow(panel);
    let offset = scroll::sync(panel);
    for (index, (id, _)) in entries().enumerate() {
        let rect = geometry::button(index, crate::desktop::tool_commands::TOOLS.len(), dpi);
        if let Ok(button) = GetDlgItem(Some(panel), id as i32) {
            if matches(button, panel, rect, offset) {
                continue;
            }
            // Read failures retain existing best-effort positioning; caches must not swallow actual resizing/scrolling.
            let _ = SetWindowPos(
                button,
                None,
                rect.left - offset.x,
                rect.top - offset.y,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}
