//! Read-only hit tests in the bound source's MSAA tree before/after scrolling encourage Chromium to refresh delayed AX position feedback.
//! No desktop ElementFromPoint, focus changes, or input sends; blocking is still terminated by the original constrained child's action budget.
use super::Target;
use crate::live_window_target::input_target::Destination;
use windows::{
    core::{Interface, Result, BOOL},
    Win32::{
        Foundation::*,
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

pub(super) fn position(target: &Target, point: [i32; 2]) -> Result<()> {
    target.validate()?;
    let destination = refresh_destination(target, point)?;
    destination
        .preflight()
        .map_err(|_| windows::core::Error::from(E_ABORT))?;
    unsafe {
        let mut raw = std::ptr::null_mut();
        AccessibleObjectFromWindow(
            destination.hwnd(),
            windows::Win32::UI::WindowsAndMessaging::OBJID_CLIENT.0 as u32,
            &IAccessible::IID,
            &mut raw,
        )?;
        if raw.is_null() {
            return Err(E_POINTER.into());
        }
        let accessible = IAccessible::from_raw(raw);
        // Request only source-local hit tests, without consuming returned nodes or reading text. Interfaces and VARIANTs are released through RAII.
        let _hit = accessible.accHitTest(point[0], point[1])?;
        Ok(())
    }
}

// Chromium's page AX tree belongs to the renderer HWND; top-level client-area hit tests may refresh only the browser shell.
// Renderers may not participate in ordinary visible-window hit testing, so read-only refresh cannot reuse pointer visibility filtering.
fn refresh_destination(target: &Target, point: [i32; 2]) -> Result<Destination> {
    let mut query = Query {
        point,
        selected: None,
        remaining: 256,
        failed: false,
    };
    unsafe {
        // The callback borrows only this stack frame; no desktop access or retention of windows/pointers in asynchronous tasks.
        let _ = EnumChildWindows(
            Some(target.hwnd()),
            Some(visit),
            LPARAM(&mut query as *mut _ as isize),
        );
    }
    if query.failed {
        return Err(E_ABORT.into());
    }
    Destination::bind(target, query.selected.unwrap_or(target.hwnd())).map_err(|_| E_ABORT.into())
}

struct Query {
    point: [i32; 2],
    selected: Option<HWND>,
    remaining: usize,
    failed: bool,
}

unsafe extern "system" fn visit(hwnd: HWND, param: LPARAM) -> BOOL {
    let query = &mut *(param.0 as *mut Query);
    if query.remaining == 0 {
        query.failed = true;
        return false.into();
    }
    query.remaining -= 1;
    let mut class = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut class) as usize;
    if !class[..n]
        .iter()
        .copied()
        .eq("Chrome_RenderWidgetHostHWND".encode_utf16())
    {
        return true.into();
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() {
        query.failed = true;
        return false.into();
    }
    if contains(rect, query.point) {
        // Ownership of overlapping renderers is ambiguous; enumeration order must not select a page.
        if query.selected.is_some() {
            query.failed = true;
            return false.into();
        }
        query.selected = Some(hwnd);
    }
    true.into()
}

fn contains(rect: RECT, point: [i32; 2]) -> bool {
    point[0] >= rect.left && point[0] < rect.right && point[1] >= rect.top && point[1] < rect.bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;
    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        DefWindowProcW(hwnd, msg, wp, lp)
    }
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    unsafe fn window(class: windows::core::PCWSTR, parent: Option<HWND>, x: i32) -> Window {
        Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!(""),
                if parent.is_some() { WS_CHILD } else { WS_POPUP },
                x,
                0,
                100,
                100,
                parent,
                None,
                None,
                None,
            )
            .unwrap(),
        )
    }
    #[test]
    fn renderer_refresh_is_source_scoped_and_includes_hidden_children() {
        unsafe {
            let class = w!("Chrome_RenderWidgetHostHWND");
            let registration = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                lpszClassName: class,
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&registration), 0);
            {
                let root = window(w!("STATIC"), None, 0);
                let other = window(w!("STATIC"), None, 0);
                let foreign = window(class, Some(other.0), 0);
                let target = Target::bind(root.0).unwrap();
                assert_eq!(
                    refresh_destination(&target, [10, 10]).unwrap().hwnd(),
                    root.0
                );
                let child = window(class, Some(root.0), 0);
                let mut rect = RECT::default();
                GetWindowRect(child.0, &mut rect).unwrap();
                let point = [rect.left + 1, rect.top + 1];
                assert!(!IsWindowVisible(child.0).as_bool());
                assert_eq!(refresh_destination(&target, point).unwrap().hwnd(), child.0);
                assert_ne!(child.0, foreign.0);
                assert_eq!(
                    refresh_destination(&target, [rect.right, rect.bottom])
                        .unwrap()
                        .hwnd(),
                    root.0
                );
                let overlap = window(class, Some(root.0), 0);
                assert!(refresh_destination(&target, point).is_err());
                drop(overlap);
                assert_eq!(refresh_destination(&target, point).unwrap().hwnd(), child.0);
                let mut exhausted = Query {
                    point,
                    selected: None,
                    remaining: 0,
                    failed: false,
                };
                assert!(!visit(child.0, LPARAM(&mut exhausted as *mut _ as isize)).as_bool());
                assert!(exhausted.failed);
                drop(root);
                assert!(refresh_destination(&target, point).is_err());
            }
            UnregisterClassW(class, None).unwrap();
        }
    }
}
