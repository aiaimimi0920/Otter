//! Owned hidden-HWND regressions for live selection; no desktop capture or system input.
use super::*;

fn screen() -> RECT {
    RECT {
        left: -300,
        top: -100,
        right: 0,
        bottom: 100,
    }
}
fn at(x: isize, y: isize) -> Option<LPARAM> {
    Some(LPARAM(x | (y << 16)))
}
unsafe fn state(hwnd: HWND) -> &'static Selection {
    &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection)
}
unsafe fn cancel(hwnd: HWND) {
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
}

#[test]
fn restored_region_is_adjustable_until_enter_and_escape_discards_it() {
    unsafe {
        let monitors = crate::desktop::capture_monitors::snapshot().unwrap();
        let [l, t, r, b] = monitors[0];
        let rect = [l + 10, t + 10, l + 40, t + 30];
        let screen = RECT {
            left: l,
            top: t,
            right: r,
            bottom: b,
        };
        let last = crate::capture_last_region::Last::new(rect, monitors).unwrap();
        for accept in [true, false] {
            let result = select_region(screen, vec![], Some(last.clone()), false, |hwnd| {
                let s = state(hwnd);
                s.hud.install(hwnd).unwrap();
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(b'R' as usize)), None);
                assert!(s.adjusting.get() && !s.done.get());
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RIGHT.0 as usize)), None);
                let dx = if GetKeyState(VK_SHIFT.0 as i32) < 0 {
                    10
                } else {
                    1
                };
                assert_eq!(s.model.get().rect.unwrap().left, 10 + dx);
                SendMessageW(
                    hwnd,
                    WM_KEYDOWN,
                    Some(WPARAM(
                        if accept { VK_RETURN } else { VK_ESCAPE }.0 as usize,
                    )),
                    None,
                );
            })
            .unwrap();
            assert_eq!(result.is_some(), accept);
            if let Some(r) = result {
                assert_eq!((r.top, r.bottom, r.right - r.left), (rect[1], rect[3], 30));
            }
        }
    }
}

#[test]
fn transparent_selection_returns_release_bounds_and_destroys_both_layers() {
    let input = Cell::new(HWND::default());
    let outline = Cell::new(HWND::default());
    let result = select_with(screen(), Vec::new(), false, |hwnd| unsafe {
        input.set(hwnd);
        outline.set(state(hwnd).outline.get());
        let mut alpha = 0;
        GetLayeredWindowAttributes(hwnd, None, Some(&mut alpha), None).unwrap();
        assert_eq!(alpha, 1);
        let style = GetWindowLongPtrW(outline.get(), GWL_EXSTYLE) as u32;
        assert_ne!(style & WS_EX_TRANSPARENT.0, 0);
        for window in [hwnd, outline.get()] {
            // Check the Shell's actual property name on both HWNDs before their first show.
            assert!(!IsWindowVisible(window).as_bool());
            assert_eq!(GetPropW(window, w!("NonRudeHWND")).0 as usize, 1);
            let mut affinity = 0;
            GetWindowDisplayAffinity(window, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        }
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(20, 30));
        SendMessageW(hwnd, WM_MOUSEMOVE, None, at(100, 80));
        assert!(!state(hwnd).done.get());
        SendMessageW(hwnd, WM_LBUTTONUP, None, at(120, 90));
        assert!(state(hwnd).done.get() && state(hwnd).accepted.get());
        // Terminal state rejects further drag selections/cancellation and still submits the first release rectangle.
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(1, 1));
        SendMessageW(hwnd, WM_LBUTTONUP, None, at(2, 2));
        cancel(hwnd);
    })
    .unwrap()
    .unwrap();
    assert_eq!(
        result,
        RECT {
            left: -280,
            top: -70,
            right: -180,
            bottom: -10
        }
    );
    unsafe {
        assert!(!IsWindow(Some(input.get())).as_bool());
        assert!(!IsWindow(Some(outline.get())).as_bool());
    }
}

#[test]
fn interrupted_and_zero_width_drags_do_not_commit_but_fresh_reverse_drag_does() {
    for message in [WM_CANCELMODE, WM_CAPTURECHANGED, 0] {
        let result = select_with(screen(), vec![screen()], false, |hwnd| unsafe {
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(10, 10));
            SendMessageW(hwnd, WM_MOUSEMOVE, None, at(60, 60));
            if message == WM_CAPTURECHANGED {
                ReleaseCapture().unwrap();
            } else if message != 0 {
                SendMessageW(hwnd, message, None, None);
            }
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(10, 80));
            assert!(!state(hwnd).done.get());
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
            assert!(!state(hwnd).done.get());
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(90, 70));
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(50, 40));
            cancel(hwnd);
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            result,
            RECT {
                left: -250,
                top: -60,
                right: -210,
                bottom: -30
            }
        );
    }
}

#[test]
fn cancellation_display_change_and_close_never_return_stale_bounds() {
    for message in [WM_KEYDOWN, WM_RBUTTONDOWN, WM_CLOSE, WM_DISPLAYCHANGE] {
        let result = select_with(screen(), Vec::new(), false, |hwnd| unsafe {
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(20, 30));
            SendMessageW(hwnd, WM_MOUSEMOVE, None, at(100, 80));
            SendMessageW(hwnd, message, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(120, 90));
        })
        .unwrap();
        assert!(result.is_none());
        unsafe {
            assert!(GetCapture().is_invalid());
        }
    }
}

#[test]
fn click_keeps_window_selection_but_empty_desktop_click_does_not_capture() {
    for window in [false, true] {
        let result = select_with(
            screen(),
            if window { vec![screen()] } else { Vec::new() },
            false,
            |hwnd| unsafe {
                SendMessageW(hwnd, WM_MOUSEMOVE, None, at(20, 30));
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(20, 30));
                SendMessageW(hwnd, WM_LBUTTONUP, None, at(20, 30));
                assert_eq!(state(hwnd).accepted.get(), window);
                cancel(hwnd);
            },
        )
        .unwrap();
        assert_eq!(result, window.then(screen));
    }
}

#[test]
fn invalid_desktop_bounds_never_create_selector() {
    for screen in [
        RECT::default(),
        RECT {
            left: i32::MIN,
            right: i32::MAX,
            ..screen()
        },
    ] {
        assert!(select_with(screen, Vec::new(), false, |_| panic!(
            "Invalid bounds must not create windows"
        ))
        .is_err());
    }
}

#[test]
fn outline_paints_only_decorations_and_keeps_selection_interior_transparent() {
    let result = select_with(screen(), Vec::new(), false, |hwnd| unsafe {
        let bitmap = crate::software_dib::Bitmap::new(
            windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                width: 300,
                height: 200,
            },
        )
        .unwrap();
        let state = state(hwnd);
        let area = RECT {
            left: 0,
            top: 0,
            right: 300,
            bottom: 200,
        };
        for x in [50, 70] {
            let mut model = state.model.get();
            model.rect = Some(RECT {
                left: x,
                top: 100,
                right: x + 100,
                bottom: 190,
            });
            state.model.set(model);
            paint::render(state.outline.get(), state, bitmap.dc(), &area).unwrap();
            GdiFlush().ok().unwrap();
            assert_eq!(GetPixel(bitmap.dc(), x, 100), paint::KEY);
            assert_eq!(GetPixel(bitmap.dc(), x + 99, 189), paint::KEY);
            assert_eq!(GetPixel(bitmap.dc(), x - 1, 150), COLORREF(0x38ffd9));
            assert_eq!(
                GetPixel(bitmap.dc(), 49, 150),
                if x == 50 {
                    COLORREF(0x38ffd9)
                } else {
                    paint::KEY
                }
            );
        }
        cancel(hwnd);
    })
    .unwrap();
    assert!(result.is_none());
}
