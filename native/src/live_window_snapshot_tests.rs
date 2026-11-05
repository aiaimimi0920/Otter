use super::*;
use windows::core::w;

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
unsafe fn window(style: WINDOW_EX_STYLE) -> Window {
    Window(
        CreateWindowExW(
            style,
            w!("STATIC"),
            w!("snapshot fixture"),
            WS_POPUP,
            25,
            35,
            80,
            60,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}

#[test]
fn plain_hidden_window_snapshot_preserves_exact_style_geometry_and_identity() {
    unsafe {
        let window = window(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
        let target = Target::bind(window.0).unwrap();
        let snapshot = Snapshot::read(&target).unwrap();
        assert_eq!(
            snapshot.extended_style,
            GetWindowLongPtrW(window.0, GWL_EXSTYLE)
        );
        assert!(snapshot.layered.is_none());
        assert!(!snapshot.iconic);
        assert_eq!(snapshot.target.hwnd(), window.0);
        assert_eq!(
            (
                snapshot.rect.left,
                snapshot.rect.top,
                snapshot.rect.right,
                snapshot.rect.bottom
            ),
            (25, 35, 105, 95)
        );
        assert_eq!(
            snapshot.placement.length as usize,
            std::mem::size_of::<WINDOWPLACEMENT>()
        );
        assert!(!IsWindowVisible(window.0).as_bool());
        drop(window);
        assert!(snapshot.target.validate().is_err());
        assert!(Snapshot::read(&target).is_err());
    }
}

#[test]
fn layered_snapshot_preserves_color_alpha_flags_and_does_not_modify_source() {
    unsafe {
        let window = window(WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
        let flags = LWA_ALPHA | LWA_COLORKEY;
        SetLayeredWindowAttributes(window.0, COLORREF(0x123456), 97, flags).unwrap();
        let target = Target::bind(window.0).unwrap();
        for _ in 0..2 {
            let snapshot = Snapshot::read(&target).unwrap();
            let attrs = snapshot.layered.unwrap();
            assert_eq!(attrs.color, COLORREF(0x123456));
            assert_eq!(attrs.alpha, 97);
            assert_eq!(attrs.flags, flags);
            assert_eq!(
                snapshot.extended_style,
                GetWindowLongPtrW(window.0, GWL_EXSTYLE)
            );
            assert!(!IsWindowVisible(window.0).as_bool());
        }
    }
}

#[test]
fn unreadable_layered_attributes_reject_snapshot_without_changing_style() {
    unsafe {
        // 尚未调用 SetLayeredWindowAttributes，不能推测该 layered 窗口的透明属性。
        let window = window(WS_EX_LAYERED | WS_EX_TOOLWINDOW);
        let target = Target::bind(window.0).unwrap();
        let before = GetWindowLongPtrW(window.0, GWL_EXSTYLE);
        assert!(Snapshot::read(&target).is_err());
        assert_eq!(GetWindowLongPtrW(window.0, GWL_EXSTYLE), before);
        assert!(!IsWindowVisible(window.0).as_bool());
        assert!(target.validate().is_ok());
    }
}
#[test]
fn restore_preserves_original_layered_attributes_style_and_geometry() {
    unsafe {
        for (layered, topmost) in [(false, false), (true, false), (false, true), (true, true)] {
            let style = WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | if layered {
                    WS_EX_LAYERED
                } else {
                    WINDOW_EX_STYLE::default()
                };
            let style = style
                | if topmost {
                    WS_EX_TOPMOST
                } else {
                    WINDOW_EX_STYLE::default()
                };
            let window = window(style);
            if layered {
                SetLayeredWindowAttributes(
                    window.0,
                    COLORREF(0x123456),
                    97,
                    LWA_ALPHA | LWA_COLORKEY,
                )
                .unwrap();
            }
            let snapshot = Snapshot::read(&Target::bind(window.0).unwrap()).unwrap();
            SetWindowLongPtrW(
                window.0,
                GWL_EXSTYLE,
                snapshot.extended_style | WS_EX_LAYERED.0 as isize | WS_EX_TRANSPARENT.0 as isize,
            );
            SetLayeredWindowAttributes(window.0, COLORREF(0), 1, LWA_ALPHA).unwrap();
            SetWindowPos(
                window.0,
                Some(HWND_NOTOPMOST),
                100,
                120,
                20,
                30,
                SWP_NOACTIVATE,
            )
            .unwrap();
            for _ in 0..2 {
                snapshot.restore().unwrap();
                let restored = Snapshot::read(&snapshot.target).unwrap();
                assert_eq!(restored.extended_style, snapshot.extended_style);
                assert_eq!(
                    (
                        restored.rect.left,
                        restored.rect.top,
                        restored.rect.right,
                        restored.rect.bottom
                    ),
                    (25, 35, 105, 95)
                );
                assert!(!restored.visible);
                assert!(!IsWindowVisible(window.0).as_bool());
                if layered {
                    let attrs = restored.layered.unwrap();
                    assert_eq!(attrs.color, COLORREF(0x123456));
                    assert_eq!(attrs.alpha, 97);
                    assert_eq!(attrs.flags, LWA_ALPHA | LWA_COLORKEY);
                } else {
                    assert!(restored.layered.is_none());
                }
            }
        }
    }
}

#[test]
fn invalid_snapshot_and_destroyed_target_are_rejected_before_style_mutation() {
    unsafe {
        let window = window(WS_EX_TOOLWINDOW);
        let mut snapshot = Snapshot::read(&Target::bind(window.0).unwrap()).unwrap();
        let style = snapshot.extended_style;
        let original = snapshot.rect;
        snapshot.extended_style |= WS_EX_TRANSPARENT.0 as isize;
        snapshot.rect.left = i32::MIN;
        snapshot.rect.right = i32::MAX;
        assert!(snapshot.restore().is_err());
        assert_eq!(GetWindowLongPtrW(window.0, GWL_EXSTYLE), style);
        snapshot.rect = original;
        snapshot.placement.length = 0;
        assert!(snapshot.restore().is_err());
        assert_eq!(GetWindowLongPtrW(window.0, GWL_EXSTYLE), style);
        snapshot.placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        drop(window);
        assert!(snapshot.restore().is_err());
    }
}

#[test]
fn zero_extended_style_is_valid_not_an_implicit_win32_failure() {
    unsafe {
        let window = window(WINDOW_EX_STYLE::default());
        assert_eq!(GetWindowLongPtrW(window.0, GWL_EXSTYLE), 0);
        SetLastError(ERROR_ACCESS_DENIED);
        let snapshot = Snapshot::read(&Target::bind(window.0).unwrap()).unwrap();
        assert_eq!(snapshot.extended_style, 0);
        assert!(snapshot.layered.is_none());
    }
}
