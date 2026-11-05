use super::*;
#[test]
fn normalized_client_endpoints_and_single_pixel_do_not_overshoot() {
    let rect = RECT {
        left: -100,
        top: 10,
        right: 100,
        bottom: 110,
    };
    assert_eq!(rectangle_point(rect, [0.0, 0.0]), Ok([-100, 10]));
    assert_eq!(rectangle_point(rect, [1.0, 1.0]), Ok([99, 109]));
    assert_eq!(rectangle_point(rect, [0.5, 0.5]), Ok([0, 60]));
    assert_eq!(
        rectangle_point(
            RECT {
                left: 3,
                top: 5,
                right: 4,
                bottom: 6
            },
            [0.5; 2]
        ),
        Ok([3, 5])
    );
    assert_eq!(
        rectangle_point(RECT::default(), [0.0; 2]),
        Err(Error::InvalidBounds)
    );
    for invalid in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        assert_eq!(
            rectangle_point(rect, [invalid, 0.0]),
            Err(Error::InvalidPoint)
        );
    }
}
#[test]
fn physical_crop_checks_frame_membership_before_mapping() {
    let bounds = RECT {
        left: -200,
        top: -100,
        right: 200,
        bottom: 200,
    };
    let region = FrameRegion {
        left: 20,
        top: 10,
        width: 100,
        height: 50,
    };
    assert_eq!(region_point(bounds, region, [0.0; 2]), Ok([-180, -90]));
    assert_eq!(region_point(bounds, region, [1.0; 2]), Ok([-81, -41]));
    assert!(region_point(
        bounds,
        FrameRegion {
            left: u32::MAX,
            ..region
        },
        [0.0; 2]
    )
    .is_err());
    assert!(region_point(bounds, FrameRegion { width: 0, ..region }, [0.0; 2]).is_err());
    assert!(region_point(
        bounds,
        FrameRegion {
            width: 381,
            ..region
        },
        [0.0; 2]
    )
    .is_err());
}
#[test]
#[ignore = "requires interactive default desktop; creates owned hidden windows without input"]
fn actual_client_child_transform_and_dpi_context_restore() {
    use windows::core::w;
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _fixture_dpi = Dpi(old);
        let before = GetThreadDpiAwarenessContext();
        let root = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
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
            WS_CHILD,
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
        let target = Target::bind(root.0).unwrap();
        let destination = Destination::bind(&target, child).unwrap();
        let mapped = map(&target, &destination, [1.0; 2], None).unwrap();
        assert_eq!(mapped.root_client, [199, 99]);
        assert_eq!(mapped.destination_client, [179, 89]);
        let cropped = map(
            &target,
            &destination,
            [1.0; 2],
            Some(FrameRegion {
                left: 20,
                top: 10,
                width: 80,
                height: 40,
            }),
        )
        .unwrap();
        assert_eq!(cropped.root_client, [99, 49]);
        assert_eq!(cropped.destination_client, [79, 39]);
        assert!(AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool());
        assert!(map(
            &target,
            &destination,
            [0.0; 2],
            Some(FrameRegion {
                left: u32::MAX,
                top: 0,
                width: 1,
                height: 1
            })
        )
        .is_err());
        assert!(AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool());
        drop(root);
        assert_eq!(
            map(&target, &destination, [0.5; 2], None),
            Err(Error::TargetChanged)
        );
    }
}
