//! 真实窗口 DPI awareness 矩阵；只验证当前显示器，不冒充多屏不同缩放验收。
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

#[test]
#[ignore = "interactive desktop; owned alpha=1 source windows across DPI awareness contexts"]
fn actual_source_awareness_matrix_maps_crop_and_restores_caller_context() {
    unsafe {
        let contexts = [
            DPI_AWARENESS_CONTEXT_UNAWARE,
            DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ];
        for source_context in contexts {
            let old = SetThreadDpiAwarenessContext(source_context);
            assert!(!old.0.is_null());
            let _source_dpi = Dpi(old);
            let root = Window(
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    w!("STATIC"),
                    w!("Otter DPI fixture"),
                    WS_POPUP,
                    100,
                    100,
                    200,
                    100,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let child = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                20,
                10,
                80,
                40,
                Some(root.0),
                None,
                None,
                None,
            )
            .unwrap();
            SetLayeredWindowAttributes(root.0, COLORREF(0), 1, LWA_ALPHA).unwrap();
            let _ = ShowWindow(root.0, SW_SHOWNOACTIVATE);
            let target = Target::bind(root.0).unwrap();
            let destination = Destination::bind(&target, child).unwrap();
            let bounds = frame_bounds(root.0).unwrap();
            let region = FrameRegion {
                left: 0,
                top: 0,
                width: (bounds.right - bounds.left) as u32,
                height: (bounds.bottom - bounds.top) as u32,
            };
            for caller_context in contexts {
                let old = SetThreadDpiAwarenessContext(caller_context);
                assert!(!old.0.is_null());
                let _caller_dpi = Dpi(old);
                let before = GetThreadDpiAwarenessContext();
                for point in [[0.0, 0.0], [0.5, 0.5], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]] {
                    let mapped = map(&target, &destination, point, None).unwrap();
                    let expected = [
                        (199.0_f64 * point[0]).round() as i32,
                        (99.0_f64 * point[1]).round() as i32,
                    ];
                    assert_eq!(mapped.root_client, expected);
                    assert_eq!(
                        mapped.destination_client,
                        [expected[0] - 20, expected[1] - 10]
                    );
                    let crop = map(&target, &destination, point, Some(region)).unwrap();
                    for axis in 0..2 {
                        assert!(crop.root_client[axis].abs_diff(mapped.root_client[axis]) <= 1,
                        "source={source_context:?} caller={caller_context:?} crop={crop:?} normal={mapped:?}");
                        assert!(
                            crop.destination_client[axis].abs_diff(mapped.destination_client[axis])
                                <= 1
                        );
                    }
                }
                assert!(
                    AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool()
                );
                assert!(map(
                    &target,
                    &destination,
                    [0.5; 2],
                    Some(FrameRegion {
                        width: u32::MAX,
                        ..region
                    })
                )
                .is_err());
                assert!(
                    AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool()
                );
            }
            eprintln!("native-input-dpi: source={source_context:?} dpi={} callers=3 crop=true restored=true", GetDpiForWindow(root.0));
        }
    }
}
