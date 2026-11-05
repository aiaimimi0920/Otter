//! 真实隐藏 HWND 的物理客户区和首帧定位；不捕获桌面，不冒充跨显示器人工验收。
use super::*;
use crate::{desktop::Apartment, live_frames::Source, pin_geometry::PinRect};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
#[test]
fn live_placement_hidden_client_and_first_gpu_frame_match_physical_selection() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        for (x, y, width, height) in [
            (200, 160, 320, 320),
            (-600, -300, 240, 240),
            (-30000, -30000, 128, 128),
            (100, 100, 1, 1),
            (100, 100, 1, 256),
            (100, 100, 256, 1),
        ] {
            let source = Source::new();
            let screen = RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            };
            let window = LiveWindow::new_at(source.subscribe().unwrap(), screen).unwrap();
            assert!(!IsWindowVisible(window.hwnd).as_bool());
            let expected = PinRect {
                x,
                y,
                width: width as u32,
                height: height as u32,
            };
            assert_eq!(
                window.freeze_rect([width as u32, height as u32]).unwrap(),
                expected
            );
            let mut affinity = 0;
            GetWindowDisplayAffinity(window.hwnd, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            if width == height {
                super::super::tests::publish(&source);
                SendMessageW(window.hwnd, WM_PAINT, None, None);
                assert!(window
                    .state
                    .host
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .presented_viewport([width as u32, height as u32])
                    .is_some());
                assert_eq!(window.freeze_rect([2, 2]).unwrap(), expected);
            }
            drop(window);
            assert!(!source.has_consumers());
        }
    }
}
#[test]
fn live_placement_invalid_bounds_drop_consumer_without_starting_a_window() {
    let _apartment = Apartment::new().unwrap();
    for rect in [
        RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 128,
        },
        RECT {
            left: 1,
            top: 0,
            right: 0,
            bottom: 128,
        },
        RECT {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 128,
        },
        RECT {
            left: 0,
            top: 0,
            right: 8193,
            bottom: 128,
        },
        RECT {
            left: 0,
            top: 0,
            right: 8192,
            bottom: 8192,
        },
    ] {
        let source = Source::new();
        assert!(LiveWindow::new_at(source.subscribe().unwrap(), rect).is_err());
        assert!(!source.has_consumers());
    }
    let screen = RECT {
        left: i32::MIN,
        top: 0,
        right: i32::MIN + 128,
        bottom: 128,
    };
    assert!(outer(
        screen,
        RECT {
            left: -8,
            top: -30,
            right: 136,
            bottom: 136
        }
    )
    .is_err());
}
#[test]
fn live_placement_failure_after_creation_releases_hwnd_and_consumer() {
    let _apartment = Apartment::new().unwrap();
    let source = Source::new();
    let hwnd = Cell::new(HWND::default());
    let rect = RECT {
        left: -30000,
        top: -30000,
        right: -29872,
        bottom: -29872,
    };
    assert!(
        LiveWindow::create_at_with(source.subscribe().unwrap(), rect, |window, _| {
            hwnd.set(window.hwnd);
            assert!(window.is_alive());
            Err(Error::new(
                E_FAIL,
                "injected placement failure after HWND creation",
            ))
        })
        .is_err()
    );
    assert!(!hwnd.get().0.is_null());
    assert!(!unsafe { IsWindow(Some(hwnd.get())).as_bool() });
    assert!(!source.has_consumers());
}
#[test]
fn live_placement_nonclient_math_preserves_physical_pixels_at_multiple_dpis() {
    unsafe {
        for dpi in [96, 144, 192] {
            let mut inset = RECT {
                left: 0,
                top: 0,
                right: 320,
                bottom: 240,
            };
            AdjustWindowRectExForDpi(
                &mut inset,
                WS_OVERLAPPEDWINDOW,
                false,
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                dpi,
            )
            .unwrap();
            let screen = RECT {
                left: -500,
                top: -200,
                right: -180,
                bottom: 40,
            };
            let frame = outer(screen, inset).unwrap();
            assert_eq!(frame.left - inset.left, screen.left);
            assert_eq!(frame.top - inset.top, screen.top);
            assert_eq!(frame.right - (inset.right - 320), screen.right);
            assert_eq!(frame.bottom - (inset.bottom - 240), screen.bottom);
        }
    }
}
