//! Verify endpoint semicircles, cross-pin occlusion, and actual regions/mouse hits after geometry messages on isolated desktops.
use super::*;

pub(super) unsafe fn contains(hwnd: HWND, point: Point) -> bool {
    let region = CreateRectRgn(0, 0, 0, 0);
    assert!(!region.is_invalid());
    assert_ne!(GetWindowRgn(hwnd, region), RGN_ERROR);
    let mut point = POINT {
        x: point.x,
        y: point.y,
    };
    ScreenToClient(hwnd, &mut point).ok().unwrap();
    let result = PtInRegion(region, point.x, point.y).as_bool();
    DeleteObject(HGDIOBJ(region.0)).ok().unwrap();
    result
}

#[test]
fn both_ports_keep_only_outer_half_at_scaled_negative_and_minified_geometry() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let host = app.documents[0].window.0;
        for (x, y, dpi, minified) in [
            (240, 180, 96, false),
            (-320, -180, 144, false),
            (-640, -300, 192, true),
        ] {
            SetWindowPos(host, None, x, y, 160, 120, SWP_NOZORDER | SWP_NOACTIVATE).unwrap();
            {
                // Use physical client areas with DPI scalars, independent of the test machine's actual multi-monitor/scaling settings.
                let mut nodes = ui.nodes.borrow_mut();
                let node = nodes.iter_mut().find(|node| node.endpoint == 0).unwrap();
                node.dpi = dpi;
                node.minified = minified;
            }
            window::refresh(&ui).unwrap();
            for side in [Side::Input, Side::Output] {
                let (strip, port, outside) = port(&ui, 0, side);
                let anchor = ui.point(port).unwrap();
                let inside = Point {
                    x: 2 * anchor.x - outside.x,
                    y: outside.y,
                };
                assert!(contains(strip, outside), "{dpi}/{minified:?}/{side:?}");
                assert!(!contains(strip, inside), "{dpi}/{minified:?}/{side:?}");
                // New capsule centers lie on white-border outer edges; the right exclusive boundary itself is outside the white border.
                // Verify actual border pixels are fully subtracted; old dot centers do not substitute for the occlusion contract.
                let node = ui.node(0).unwrap();
                let border_pixel = Point {
                    x: if side == Side::Input {
                        node.rect.left - 1
                    } else {
                        node.rect.right
                    },
                    y: anchor.y,
                };
                assert!(border_covers(&ui, 0, border_pixel));
                assert!(!contains(strip, border_pixel));
                let padding = desktop::pin_border_style::width(dpi) as i32;
                for dx in -padding..=padding {
                    let point = Point {
                        x: anchor.x + dx,
                        y: anchor.y,
                    };
                    if border_covers(&ui, 0, point) {
                        assert!(!contains(strip, point));
                    }
                }
            }
        }
        app.shutdown();
    }
}

unsafe fn place(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    SetWindowPos(
        hwnd,
        None,
        x,
        y,
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    assert_eq!(
        scene::frame(hwnd).unwrap(),
        Rect {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height
        },
        "fixture must position the image client area, not a captioned frame"
    );
    UpdateWindow(hwnd).ok().unwrap();
}

#[test]
fn every_pin_occludes_ports_and_move_resize_hide_or_destroy_restores_outer_half() {
    unsafe {
        for side in [Side::Input, Side::Output] {
            let mut app = visible_app();
            for doc in &app.documents {
                // The original app fixture is an empty captioned document; switch to borderless pin styles so geometry matches content.
                SetWindowLongPtrW(doc.window.0, GWL_STYLE, (WS_POPUP | WS_VISIBLE).0 as isize);
                SetWindowPos(
                    doc.window.0,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
                UpdateWindow(doc.window.0).ok().unwrap();
            }
            let ui = app.chrome.ui.clone();
            let blocker = app.documents[1].window.0;
            let (strip, _, point) = port(&ui, 0, side);
            let hit = || {
                WindowFromPoint(POINT {
                    x: point.x,
                    y: point.y,
                })
            };
            assert!(contains(strip, point));
            assert_eq!(hit(), strip);
            // The endpoint host stays put; another pin must occlude it even without Z-order changes.
            place(blocker, point.x - 20, point.y - 20, 40, 40);
            assert!(!contains(strip, point));
            assert_eq!(hit(), blocker);
            place(blocker, point.x - 20, point.y - 20, 40, 10);
            assert!(contains(strip, point), "resize uncovers port");
            assert_eq!(hit(), strip);
            place(blocker, point.x - 20, point.y - 20, 40, 40);
            assert!(!contains(strip, point));
            place(blocker, point.x + 80, point.y - 20, 40, 40);
            assert!(contains(strip, point), "move uncovers port");
            assert_eq!(hit(), strip);
            place(blocker, point.x - 20, point.y - 20, 40, 40);
            let _ = ShowWindow(blocker, SW_HIDE);
            assert!(contains(strip, point), "hidden pin does not occlude");
            let _ = ShowWindow(blocker, SW_SHOWNOACTIVATE);
            assert!(!contains(strip, point));
            DestroyWindow(blocker).unwrap();
            assert!(contains(strip, point), "destroyed pin does not occlude");
            assert_eq!(hit(), strip);
            app.shutdown();
        }
    }
}
