//! Real hidden-HWND message verification for frozen-frame selection; no desktop reads or system input injection.
use super::*;
fn image(width: u32, height: u32) -> Image {
    Image {
        width,
        height,
        pixels: vec![255; (width * height * 4) as usize].into(),
    }
}
#[test]
fn hidden_native_region_selects_synthetic_pixels_and_cancels_without_document() {
    for cancel in [false, true] {
        let image = Image {
            width: 4,
            height: 4,
            pixels: (0..64).collect::<Vec<u8>>().into(),
        };
        let result = select_positioned_with(
            image,
            POINT { x: -1920, y: -1080 },
            Vec::new(),
            false,
            |hwnd| unsafe {
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(1 | (1 << 16))));
                if cancel {
                    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
                }
                SendMessageW(hwnd, WM_LBUTTONUP, None, Some(LPARAM(3 | (3 << 16))));
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
                assert!(state.done.get());
                assert_eq!(state.accepted.get(), !cancel);
                // Extra input after completion must neither adjust nor resubmit the finalized rectangle.
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(0)));
                SendMessageW(hwnd, WM_LBUTTONUP, None, Some(LPARAM(1 | (1 << 16))));
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap();
        if cancel {
            assert!(result.is_none());
        } else {
            let (image, origin) = result.unwrap();
            assert_eq!((origin.x, origin.y), (-1919, -1079));
            assert_eq!((image.width, image.height), (2, 2));
            assert_eq!(&image.pixels[..8], &[20, 21, 22, 23, 24, 25, 26, 27]);
        }
    }
    for click in [false, true] {
        let windows = vec![
            RECT {
                left: 20,
                top: 20,
                right: 70,
                bottom: 60,
            },
            RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
        ];
        let selected = select_with(
            image(100, 100),
            POINT::default(),
            windows,
            false,
            |hwnd| unsafe {
                let point = Some(LPARAM(40 | (40 << 16)));
                SendMessageW(hwnd, WM_MOUSEMOVE, None, point);
                if click {
                    SendMessageW(hwnd, WM_LBUTTONDOWN, None, point);
                    SendMessageW(hwnd, WM_LBUTTONUP, None, point);
                }
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
                assert_eq!(state.accepted.get(), click);
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap();
        if click {
            let selected = selected.unwrap();
            assert_eq!((selected.width, selected.height), (50, 40));
        } else {
            assert!(selected.is_none());
        }
    }
}
#[test]
fn live_drag_and_double_click_share_selector_without_cropping_frozen_pixels() {
    let image = image(100, 100);
    let original = image.pixels.clone();
    for whole in [false, true] {
        let rect = RECT {
            left: 20,
            top: 20,
            right: 80,
            bottom: 70,
        };
        let result = select_bounds_with(
            &image,
            POINT {
                x: -30000,
                y: -30000,
            },
            vec![rect],
            false,
            Mode::Live,
            |hwnd| unsafe {
                let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(30, 30));
                SendMessageW(
                    hwnd,
                    WM_LBUTTONUP,
                    None,
                    at(if whole { 30 } else { 60 }, if whole { 30 } else { 50 }),
                );
                if whole {
                    SendMessageW(hwnd, WM_LBUTTONDBLCLK, None, at(30, 30));
                }
                // If the official completion branch breaks, cancellation fails the test rather than hanging the message loop.
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.1, whole);
        assert_eq!(
            result.0,
            if whole {
                rect
            } else {
                RECT {
                    left: 30,
                    top: 30,
                    right: 60,
                    bottom: 50,
                }
            }
        );
        assert!(image.pixels.same_storage(&original));
    }
    let result = select_bounds_with(
        &image,
        POINT {
            x: -30000,
            y: -30000,
        },
        vec![],
        false,
        Mode::Live,
        |hwnd| unsafe {
            SendMessageW(hwnd, WM_LBUTTONDBLCLK, None, Some(LPARAM(10 | (10 << 16))));
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        },
    )
    .unwrap();
    assert!(result.is_none());
}

#[test]
fn quick_capture_rejects_empty_drags_and_lost_capture_then_accepts_a_fresh_drag() {
    for interruption in [WM_CANCELMODE, WM_CAPTURECHANGED, 0] {
        let result = select_positioned_with(
            image(100, 80),
            POINT::default(),
            vec![RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 80,
            }],
            false,
            |hwnd| unsafe {
                let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(10, 10));
                SendMessageW(hwnd, WM_MOUSEMOVE, None, at(30, 30));
                if interruption == WM_CAPTURECHANGED {
                    ReleaseCapture().unwrap();
                } else if interruption != 0 {
                    SendMessageW(hwnd, interruption, None, None);
                }
                // Zero-width dragging must not fall back to the whole hovered window or submit the pre-interruption rectangle.
                SendMessageW(hwnd, WM_LBUTTONUP, None, at(10, 40));
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
                assert!(!state.done.get() && !state.accepted.get());
                assert!(state.model.get().rect.is_none());
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
                assert!(!state.done.get());
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(90, 70));
                SendMessageW(hwnd, WM_LBUTTONUP, None, at(60, 40));
                assert!(state.done.get() && state.accepted.get());
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!((result.0.width, result.0.height), (30, 30));
        assert_eq!(result.1, POINT { x: 60, y: 40 });
    }
}

#[test]
fn pointer_motion_without_a_window_preview_has_no_magnifier_repaint() {
    let result = select_positioned_with(
        image(300, 300),
        POINT::default(),
        Vec::new(),
        false,
        |hwnd| unsafe {
            assert!(ValidateRect(Some(hwnd), None).as_bool());
            for (x, y) in [(30, 30), (280, 280), (280, 280)] {
                SendMessageW(hwnd, WM_MOUSEMOVE, None, Some(LPARAM(x | (y << 16))));
                assert!(!GetUpdateRect(hwnd, None, false).as_bool());
            }
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        },
    )
    .unwrap();
    assert!(result.is_none());
}

#[test]
fn live_interrupted_drag_never_submits_old_selection_and_can_restart() {
    for interruption in [WM_CANCELMODE, WM_CAPTURECHANGED, WM_DISPLAYCHANGE, WM_CLOSE] {
        let result = select_bounds_with(
            &image(100, 80),
            POINT::default(),
            Vec::new(),
            false,
            Mode::Live,
            |hwnd| unsafe {
                let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(10, 10));
                SendMessageW(hwnd, WM_MOUSEMOVE, None, at(30, 30));
                if interruption == WM_CAPTURECHANGED {
                    ReleaseCapture().unwrap();
                } else {
                    SendMessageW(hwnd, interruption, None, None);
                }
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
                assert!(!state.accepted.get());
                if matches!(interruption, WM_CANCELMODE | WM_CAPTURECHANGED) {
                    SendMessageW(hwnd, WM_LBUTTONUP, None, at(30, 30));
                    assert!(!state.accepted.get() && !state.done.get());
                    SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(60, 40));
                    SendMessageW(hwnd, WM_LBUTTONUP, None, at(90, 70));
                    assert!(state.accepted.get() && state.done.get());
                }
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap();
        if matches!(interruption, WM_CANCELMODE | WM_CAPTURECHANGED) {
            assert_eq!(
                result,
                Some((
                    RECT {
                        left: 60,
                        top: 40,
                        right: 90,
                        bottom: 70
                    },
                    false
                ))
            );
        } else {
            assert!(result.is_none());
        }
    }
}

#[test]
fn long_capture_keeps_adjustment_and_confirmation_in_its_own_mode() {
    let result = select_bounds_with(
        &image(100, 80),
        POINT::default(),
        Vec::new(),
        false,
        Mode::Adjust,
        |hwnd| unsafe {
            let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(10, 10));
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(60, 60));
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
            assert!(!state.done.get());
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(30, 30));
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(40, 40));
            assert!(!state.done.get());
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
            assert!(state.done.get());
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        result.0,
        RECT {
            left: 20,
            top: 20,
            right: 70,
            bottom: 70
        }
    );
}

#[test]
fn clamped_drag_and_nudge_repaint_only_when_the_visible_rectangle_changes() {
    let result = select_bounds_with(
        &image(100, 80),
        POINT::default(),
        Vec::new(),
        false,
        Mode::Adjust,
        |hwnd| unsafe {
            // Hidden HWNDs retain no update region; show without activation on an isolated desktop to observe invalidation rectangles.
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            assert!(ValidateRect(Some(hwnd), None).as_bool());
            let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection);
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(10, 10));
            SendMessageW(hwnd, WM_MOUSEMOVE, None, at(110, 90));
            assert!(GetUpdateRect(hwnd, None, false).as_bool());
            assert!(ValidateRect(Some(hwnd), None).as_bool());
            for (x, y) in [(120, 100), (130, 110)] {
                SendMessageW(hwnd, WM_MOUSEMOVE, None, at(x, y));
                assert!(!GetUpdateRect(hwnd, None, false).as_bool());
                assert_eq!(state.model.get().rect.unwrap().right, 100);
            }
            SendMessageW(hwnd, WM_MOUSEMOVE, None, at(90, 70));
            assert!(GetUpdateRect(hwnd, None, false).as_bool());
            SendMessageW(hwnd, WM_LBUTTONUP, None, at(100, 80));
            assert!(ValidateRect(Some(hwnd), None).as_bool());
            for key in [VK_RIGHT, VK_DOWN] {
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(key.0 as usize)), None);
                assert!(!GetUpdateRect(hwnd, None, false).as_bool());
            }
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_LEFT.0 as usize)), None);
            assert!(GetUpdateRect(hwnd, None, false).as_bool());
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        result.0,
        RECT {
            left: 9,
            top: 10,
            right: 99,
            bottom: 80
        }
    );
}
