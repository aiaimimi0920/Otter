//! Original Application Tab-backdrop pixels, clipping, and caching; no user-screen capture.
use super::*;

unsafe fn pixel(ui: &Ui, point: Point) -> COLORREF {
    let node = ui.node(0).unwrap();
    let backings = ui.backings.borrow();
    let backing = backings.iter().find(|b| b.matches(&node)).unwrap();
    if !IsWindowVisible(backing.hwnd).as_bool() {
        return COLORREF(0);
    }
    let mut bounds = RECT::default();
    GetWindowRect(backing.hwnd, &mut bounds).unwrap();
    if point.x < bounds.left
        || point.x >= bounds.right
        || point.y < bounds.top
        || point.y >= bounds.bottom
    {
        return COLORREF(0);
    }
    GdiFlush().ok().unwrap();
    let color = GetPixel(
        backing.surface.borrow().as_ref().unwrap().dc(),
        point.x - bounds.left,
        point.y - bounds.top,
    );
    color
}

#[test]
fn visible_tab_backing_follows_topmost_and_capture_protection_with_its_real_inline_controls() {
    unsafe {
        let mut app = app();
        let ui = app.chrome.ui.clone();
        let host = app.documents[0].window.0;
        let backing = ui.backings.borrow()[0].hwnd;
        let parent = strip(&app);
        assert!(IsWindowVisible(backing).as_bool());
        for band in [HWND_TOPMOST, HWND_NOTOPMOST, HWND_TOPMOST] {
            SetWindowPos(
                host,
                Some(band),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .unwrap();
            for hwnd in [parent, backing] {
                assert_eq!(
                    GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                    GetWindowLongW(host, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0
                );
                assert!(IsWindowVisible(hwnd).as_bool());
            }
        }
        for protected in [false, true] {
            app.documents[0]
                .state
                .capture_protection_requested
                .set(protected);
            app.sync_workspace_chrome(true);
            for hwnd in [parent, backing] {
                let mut affinity = 0;
                GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
                assert_eq!(
                    affinity,
                    if protected {
                        WDA_EXCLUDEFROMCAPTURE.0
                    } else {
                        0
                    }
                );
            }
        }
        let handles = ui.handles();
        app.shutdown();
        assert!(handles
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
        assert!(ui.backings.borrow().is_empty());
    }
}

#[test]
fn moving_a_distant_pin_keeps_the_other_tab_backing_cached_and_closing_it_preserves_the_backing() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let other = app.documents[1].window.0;
        SetWindowPos(
            other,
            None,
            1400,
            500,
            160,
            120,
            SWP_SHOWWINDOW | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let ui = app.chrome.ui.clone();
        let backing = ui.backings.borrow()[0].clone();
        let other_backing = ui.backings.borrow()[1].hwnd;
        let submissions = backing.surface.borrow().as_ref().unwrap().submissions.get();
        SetWindowPos(
            other,
            None,
            1460,
            520,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert_eq!(
            backing.surface.borrow().as_ref().unwrap().submissions.get(),
            submissions
        );
        DestroyWindow(other).unwrap();
        assert!(!IsWindowVisible(other_backing).as_bool());
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        app.chrome.windows.as_mut().unwrap().sync().unwrap();
        assert!(!IsWindow(Some(other_backing)).as_bool());
        assert!(IsWindowVisible(backing.hwnd).as_bool());
        assert_eq!(
            backing.surface.borrow().as_ref().unwrap().submissions.get(),
            submissions
        );
        drop(backing);
        app.shutdown();
    }
}
#[test]
fn tab_groups_paint_continuously_without_turning_row_gaps_into_input_or_repainting_when_idle() {
    unsafe {
        let mut app = app();
        let ui = &app.chrome.ui;
        let handles = ui.handles();
        let p = port(&app, "p1");
        let row = port_visual::layout(ui, p).unwrap();
        let point = Point {
            x: row.frame.x + row.frame.width / 2,
            y: row.frame.y - 4,
        };
        assert_eq!(pixel(ui, point), COLORREF(0x120d0a));
        let parent = strip(&app);
        let mut bounds = RECT::default();
        GetWindowRect(parent, &mut bounds).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(parent, region), RGN_ERROR);
        assert!(!PtInRegion(region, point.x - bounds.left, point.y - bounds.top).as_bool());
        DeleteObject(HGDIOBJ(region.0)).unwrap();
        assert_ne!(
            GetWindowLongW(ui.backings.borrow()[0].hwnd, GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT.0,
            0
        );
        let backing = ui.backings.borrow()[0].clone();
        let submissions = backing.surface.borrow().as_ref().unwrap().submissions.get();
        window::refresh(ui).unwrap();
        assert_eq!(
            backing.surface.borrow().as_ref().unwrap().submissions.get(),
            submissions
        );
        ui.disclosed.borrow_mut().insert(p, false);
        window::refresh(ui).unwrap();
        assert_eq!(
            pixel(ui, point),
            COLORREF(0),
            "a locally folded row breaks the background group"
        );
        assert_eq!(ui.handles(), handles);
        app.shutdown();
    }
}

#[test]
fn cached_pin_move_includes_the_visible_tab_backing_before_the_next_refresh() {
    unsafe {
        let mut app = app();
        let ui = app.chrome.ui.clone();
        let doc = &app.documents[0];
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas.pinned = true;
            canvas.tool = crate::editor::Tool::Pan;
            canvas
                .replace_image(desktop::image::Image {
                    width: 8,
                    height: 8,
                    pixels: [20, 80, 160, 255].repeat(64).into(),
                })
                .unwrap();
        }
        let host = doc.window.0;
        SendMessageW(host, WM_ENTERSIZEMOVE, None, None);
        let backing = ui.backings.borrow()[0].hwnd;
        assert!(IsWindowVisible(backing).as_bool());
        let mut before = RECT::default();
        GetWindowRect(backing, &mut before).unwrap();
        let mut rect = RECT::default();
        GetWindowRect(host, &mut rect).unwrap();
        for (dx, dy) in [(37, 11), (-30, 20)] {
            desktop::pin_window::movement::position(
                host,
                &doc.state,
                crate::pin_geometry::PinRect {
                    x: rect.left + dx,
                    y: rect.top + dy,
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                },
            );
            let mut moved = RECT::default();
            GetWindowRect(backing, &mut moved).unwrap();
            assert_eq!((moved.left, moved.top), (before.left + dx, before.top + dy));
        }
        SendMessageW(host, WM_CANCELMODE, None, None);
        app.shutdown();
    }
}
#[test]
fn backdrop_tracks_scroll_clip_and_excludes_the_image_even_when_it_is_transparent() {
    unsafe {
        let mut app = app();
        let ui = &app.chrome.ui;
        let node = ui.node(0).unwrap();
        let rail = layout(ui, &node).unwrap();
        let point = Point {
            x: rail.viewport.x + 100,
            y: rail.viewport.y + rail.viewport.height - 20,
        };
        assert_eq!(
            pixel(ui, point),
            COLORREF(0x120d0a),
            "long groups must extend their own backing bounds"
        );
        let outside = Point {
            y: rail.viewport.y + rail.viewport.height + 2,
            ..point
        };
        let mut surface_bounds = RECT::default();
        GetWindowRect(ui.backings.borrow()[0].hwnd, &mut surface_bounds).unwrap();
        // Without drawing content the surface may end at the viewport; CLR_INVALID outside DIB bounds is not a color.
        assert!(outside.y >= surface_bounds.bottom || pixel(ui, outside) == COLORREF(0));
        assert_eq!(
            pixel(
                ui,
                Point {
                    x: node.rect.left + 1,
                    y: node.rect.top + 55
                }
            ),
            COLORREF(0)
        );
        let row = port_visual::unclipped(ui, port(&app, "p0")).unwrap();
        let scroll_point = Point {
            x: row.frame.x + 100,
            y: rail.viewport.y + 10,
        };
        SendMessageW(
            strip(&app),
            WM_MOUSEWHEEL,
            Some(WPARAM(((-30i16) as u16 as usize) << 16)),
            Some(lp(scroll_point.x, scroll_point.y)),
        );
        assert_eq!(pixel(ui, point), COLORREF(0x120d0a));
        let host = app.documents[0].window.0;
        let mut before = RECT::default();
        GetWindowRect(host, &mut before).unwrap();
        SetWindowPos(
            host,
            None,
            before.left + 30,
            before.top + 20,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        assert_eq!(
            pixel(
                ui,
                Point {
                    x: point.x + 30,
                    y: point.y + 20
                }
            ),
            COLORREF(0x120d0a)
        );
        app.shutdown();
    }
}
