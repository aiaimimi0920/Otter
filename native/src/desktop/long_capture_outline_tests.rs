use super::*;
#[test]
fn border_rectangles_stay_outside_capture_at_multiple_dpis_and_reject_overflow() {
    let region = Region {
        left: -2000,
        top: -100,
        width: 800,
        height: 600,
    };
    for (dpi, thickness) in [(96, 3), (144, 5), (192, 6)] {
        let rects = rectangles(region, dpi).unwrap();
        assert_eq!(rects[0].bottom, region.top);
        assert_eq!(rects[1].top, region.top + region.height);
        assert_eq!(rects[2].right, region.left);
        assert_eq!(rects[3].left, region.left + region.width);
        assert_eq!(rects[0].bottom - rects[0].top, thickness);
        for rect in rects {
            assert!(
                rect.right <= region.left
                    || rect.left >= region.left + region.width
                    || rect.bottom <= region.top
                    || rect.top >= region.top + region.height
            );
        }
    }
    for region in [
        Region {
            left: i32::MIN,
            ..region
        },
        Region {
            left: i32::MAX - 1,
            ..region
        },
        Region { width: 0, ..region },
        Region {
            left: 0,
            width: i32::MAX,
            ..region
        },
    ] {
        assert!(rectangles(region, 96).is_err());
    }
}
#[test]
fn hidden_outline_uses_narrow_nonactivating_layered_windows_and_releases_all_handles() {
    unsafe {
        let region = Region {
            left: -100,
            top: -100,
            width: 40,
            height: 30,
        };
        // 隐藏测试不请求捕获排除，不显示覆盖窗口，也不采集真实桌面。
        let outline = Outline::new(HWND::default(), region, false, false).unwrap();
        let handles: Vec<_> = outline.windows.iter().map(|window| window.0).collect();
        assert_eq!(handles.len(), 4);
        for handle in &handles {
            assert!(!IsWindowVisible(*handle).as_bool());
            let style = GetWindowLongPtrW(*handle, GWL_EXSTYLE) as u32;
            for required in [
                WS_EX_LAYERED,
                WS_EX_TRANSPARENT,
                WS_EX_NOACTIVATE,
                WS_EX_TOOLWINDOW,
            ] {
                assert_eq!(style & required.0, required.0);
            }
        }
        let moved = Region {
            left: -90,
            top: -80,
            ..region
        };
        outline.position(moved).unwrap();
        let expected = rectangles(moved, GetDpiForWindow(handles[0]).max(96)).unwrap();
        for (handle, expected) in handles.iter().zip(expected) {
            let mut actual = RECT::default();
            GetWindowRect(*handle, &mut actual).unwrap();
            assert_eq!(actual, expected);
            assert!(!IsWindowVisible(*handle).as_bool());
        }
        drop(outline);
        assert!(handles
            .into_iter()
            .all(|handle| !IsWindow(Some(handle)).as_bool()));
    }
}
