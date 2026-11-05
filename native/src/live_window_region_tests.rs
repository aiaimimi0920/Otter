use super::*;
#[test]
#[ignore = "interactive desktop; own offscreen windows and WGC item geometry only, no capture session"]
fn actual_wgc_item_geometry_agrees_with_physical_frame_for_owned_window_styles() {
    use windows::{
        core::w,
        Graphics::Capture::GraphicsCaptureItem,
        Win32::{
            System::WinRT::{
                Graphics::Capture::IGraphicsCaptureItemInterop, RoInitialize, RoUninitialize,
                RO_INIT_MULTITHREADED,
            },
            UI::{HiDpi::*, WindowsAndMessaging::*},
        },
    };
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).unwrap();
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    RoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        struct Dpi(DPI_AWARENESS_CONTEXT);
        impl Drop for Dpi {
            fn drop(&mut self) {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        let interop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
        for style in [WS_POPUP, WS_OVERLAPPEDWINDOW] {
            let window = Window(
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("STATIC"),
                    w!("WGC crop geometry"),
                    style,
                    -30000,
                    -30000,
                    200,
                    100,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
            let target = super::super::Target::bind(window.0).unwrap();
            let bounds = target.frame_bounds().unwrap();
            let item: GraphicsCaptureItem = interop.CreateForWindow(window.0).unwrap();
            let size = item.Size().unwrap();
            eprintln!(
                "native-window-crop-item: style={} frame={}x{} item={}x{} capture-started=false",
                style.0,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                size.Width,
                size.Height
            );
            let crop = Crop::from_screen(
                bounds,
                RECT {
                    left: bounds.left + 20,
                    top: bounds.top + 10,
                    right: bounds.left + 70,
                    bottom: bounds.top + 40,
                },
            )
            .unwrap();
            assert!(crop
                .matches_texture(bounds, [size.Width as u32, size.Height as u32])
                .unwrap());
        }
    }
}
#[test]
fn crop_tracks_local_pixels_not_screen_origin_and_rejects_shrink_or_ambiguous_texture() {
    let frame = RECT {
        left: -200,
        top: -100,
        right: 0,
        bottom: 0,
    };
    let crop = Crop::from_screen(
        frame,
        RECT {
            left: -180,
            top: -90,
            right: -130,
            bottom: -60,
        },
    )
    .unwrap();
    assert_eq!(
        crop.pixels(),
        Region {
            left: 20,
            top: 10,
            width: 50,
            height: 30
        }
    );
    let moved = RECT {
        left: 300,
        top: 400,
        right: 600,
        bottom: 600,
    };
    assert_eq!(
        crop.screen(moved).unwrap(),
        RECT {
            left: 320,
            top: 410,
            right: 370,
            bottom: 440
        }
    );
    assert!(crop.matches_texture(moved, [300, 200]).unwrap());
    assert!(!crop.matches_texture(moved, [200, 100]).unwrap());
    assert!(crop
        .screen(RECT {
            right: 369,
            ..moved
        })
        .is_err());
    let extreme = RECT {
        left: i32::MIN,
        top: i32::MIN,
        right: i32::MAX,
        bottom: i32::MAX,
    };
    assert_eq!(crop.screen(extreme).unwrap().left, i32::MIN + 20);
    for screen in [
        RECT::default(),
        RECT {
            left: -201,
            ..frame
        },
        RECT { right: 1, ..frame },
        extreme,
    ] {
        assert!(Crop::from_screen(frame, screen).is_err());
    }
    let input = crop.input();
    assert_eq!(
        (input.left, input.top, input.width, input.height),
        (20, 10, 50, 30)
    );
}
#[test]
#[ignore = "requires interactive default desktop; owned hidden window coordinate mapping, no input"]
fn selected_crop_input_maps_to_same_physical_pixels_after_owned_window_moves() {
    use super::super::{input_coordinates::map, input_target::Destination, Target};
    use windows::{
        core::w,
        Win32::UI::{HiDpi::*, WindowsAndMessaging::*},
    };
    unsafe {
        struct Dpi(DPI_AWARENESS_CONTEXT);
        impl Drop for Dpi {
            fn drop(&mut self) {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("crop"),
                WS_POPUP,
                -30000,
                -30000,
                200,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let target = Target::bind(window.0).unwrap();
        let root = Destination::bind(&target, window.0).unwrap();
        let frame = target.frame_bounds().unwrap();
        let crop = Crop::from_screen(
            frame,
            RECT {
                left: frame.left + 20,
                top: frame.top + 10,
                right: frame.left + 70,
                bottom: frame.top + 40,
            },
        )
        .unwrap();
        for (x, y) in [(-30000, -30000), (-29800, -29700)] {
            SetWindowPos(
                window.0,
                None,
                x,
                y,
                200,
                100,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            for (point, expected) in [([0.0, 0.0], [20, 10]), ([1.0, 1.0], [69, 39])] {
                let actual = map(&target, &root, point, Some(crop.input())).unwrap();
                assert_eq!(actual.root_client, expected);
                assert_eq!(actual.screen, [x + expected[0], y + expected[1]]);
            }
        }
        SetWindowPos(window.0, None, 0, 0, 60, 30, SWP_NOACTIVATE | SWP_NOZORDER).unwrap();
        assert!(map(&target, &root, [0.5, 0.5], Some(crop.input())).is_err());
    }
}
