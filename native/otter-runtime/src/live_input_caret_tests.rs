//! Read-only caret observation on owned HWNDs: explicit snapshots avoid stealing real focus; full IME acceptance remains separate.
use super::*;
use windows::{core::w, Win32::Graphics::Gdi::ClientToScreen};

struct Window(HWND);
impl Window {
    fn new(parent: Option<HWND>) -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Otter caret fixture"),
                if parent.is_some() { WS_CHILD } else { WS_POPUP },
                if parent.is_some() { 20 } else { -30000 },
                if parent.is_some() { 30 } else { -30000 },
                if parent.is_some() { 50 } else { 200 },
                if parent.is_some() { 40 } else { 100 },
                parent,
                None,
                None,
                None,
            )
        }
        .unwrap();
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Self(hwnd)
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}
struct Context(DPI_AWARENESS_CONTEXT);
impl Context {
    fn enter(context: DPI_AWARENESS_CONTEXT) -> Self {
        let old = unsafe { SetThreadDpiAwarenessContext(context) };
        assert!(!old.0.is_null());
        Self(old)
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
fn info(focus: HWND, caret: HWND) -> GUITHREADINFO {
    GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        hwndFocus: focus,
        hwndCaret: caret,
        rcCaret: RECT {
            left: 4,
            top: 5,
            right: 5,
            bottom: 15,
        },
        ..Default::default()
    }
}

#[test]
fn unrelated_or_missing_focus_and_caret_are_not_admitted() {
    let root = Window::new(None);
    let child = Window::new(Some(root.0));
    let other = Window::new(None);
    let target = Target::bind(root.0).unwrap();
    assert!(in_scope(&target, &info(child.0, child.0)));
    for snapshot in [
        info(other.0, child.0),
        info(child.0, other.0),
        info(HWND::default(), child.0),
        info(child.0, HWND::default()),
    ] {
        assert!(!in_scope(&target, &snapshot));
        assert_eq!(sample_with(&target, None, || Some(snapshot)).unwrap(), None);
    }
    let mut invalid = info(child.0, child.0);
    invalid.rcCaret.bottom = invalid.rcCaret.top;
    assert!(!in_scope(&target, &invalid));
    drop(child);
    assert!(!in_scope(
        &target,
        &info(invalid.hwndFocus, invalid.hwndCaret)
    ));
}

#[test]
fn physical_caret_uses_its_own_dpi_context_then_maps_to_capture_crop() {
    let _source_context = Context::enter(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let root = Window::new(None);
    let child = Window::new(Some(root.0));
    let target = Target::bind(root.0).unwrap();
    let frame = target.frame_bounds().unwrap();
    let crop = Crop::from_screen(
        frame,
        RECT {
            left: frame.left + 10,
            top: frame.top + 10,
            right: frame.left + 110,
            bottom: frame.top + 90,
        },
    )
    .unwrap();
    let mut expected = POINT { x: 4, y: 14 };
    assert!(unsafe { ClientToScreen(child.0, &mut expected) }.as_bool());
    for context in [
        DPI_AWARENESS_CONTEXT_UNAWARE,
        DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    ] {
        let _caller_context = Context::enter(context);
        let before = unsafe { GetThreadDpiAwarenessContext() };
        let position = sample_with(&target, Some(crop), || Some(info(child.0, child.0)))
            .unwrap()
            .unwrap();
        assert_eq!(position, relative(frame, Some(crop), expected).unwrap());
        assert!(
            unsafe { AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()) }
                .as_bool()
        );
    }
}

#[test]
fn changed_caret_or_frame_during_read_is_not_published() {
    let _context = Context::enter(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let root = Window::new(None);
    let child = Window::new(Some(root.0));
    let target = Target::bind(root.0).unwrap();
    let snapshot = info(child.0, child.0);
    let mut calls = 0;
    assert_eq!(
        sample_with(&target, None, || {
            calls += 1;
            let mut current = snapshot;
            if calls == 2 {
                current.rcCaret.left += 1;
            }
            Some(current)
        })
        .unwrap(),
        None
    );
    calls = 0;
    assert_eq!(
        sample_with(&target, None, || {
            calls += 1;
            if calls == 2 {
                unsafe {
                    SetWindowPos(
                        child.0,
                        None,
                        21,
                        30,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                }
                .unwrap();
            }
            Some(snapshot)
        })
        .unwrap(),
        None
    );
    calls = 0;
    assert_eq!(
        sample_with(&target, None, || {
            calls += 1;
            if calls == 2 {
                unsafe {
                    SetWindowPos(
                        root.0,
                        None,
                        -29900,
                        -30000,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                }
                .unwrap();
            }
            Some(snapshot)
        })
        .unwrap(),
        None
    );
    drop(root);
    assert!(sample_with(&target, None, || Some(snapshot)).is_err());
}

#[test]
fn negative_screen_crop_and_outside_selection_are_not_clamped() {
    let frame = RECT {
        left: -500,
        top: -300,
        right: -100,
        bottom: -100,
    };
    let crop = Crop::from_screen(
        frame,
        RECT {
            left: -400,
            top: -250,
            right: -200,
            bottom: -150,
        },
    )
    .unwrap();
    assert_eq!(
        relative(frame, Some(crop), POINT { x: -300, y: -200 })
            .unwrap()
            .point,
        [0.5, 0.5]
    );
    assert_eq!(
        relative(frame, Some(crop), POINT { x: -450, y: -200 })
            .unwrap()
            .point,
        [-0.25, 0.5]
    );
}
