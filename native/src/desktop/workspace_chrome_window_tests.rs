//! Owned-HWND regions/capture/geometry/topology/resource gates, executed by isolated-desktop runners.
use super::*;
use windows::Win32::{
    Graphics::Gdi::*,
    UI::{Input::KeyboardAndMouse::*, Shell::*},
};
#[path = "workspace_chrome_atomic_tests.rs"]
mod atomic_tests;
#[path = "workspace_chrome_border_tests.rs"]
mod border_tests;
#[path = "workspace_chrome_close_tests.rs"]
mod close_tests;
#[path = "workspace_chrome_cursor_tests.rs"]
mod cursor_tests;
#[path = "workspace_chrome_handoff_tests.rs"]
mod handoff_tests;
#[path = "workspace_chrome_move_tests.rs"]
mod move_tests;
#[path = "workspace_node_settings_tests.rs"]
mod node_settings_tests;
#[path = "workspace_chrome_occlusion_tests.rs"]
mod occlusion;
#[path = "workspace_chrome_port_visual_tests.rs"]
mod port_visual_tests;
#[path = "workspace_chrome_refresh_tests.rs"]
mod refresh_tests;
#[path = "workspace_chrome_stack_tests.rs"]
mod stack_tests;
#[path = "workspace_chrome_stationary_tests.rs"]
mod stationary_tests;
unsafe fn visible_app() -> Application {
    let mut app = app();
    app.add_window(false).unwrap();
    for (index, doc) in app.documents.iter().enumerate() {
        SetWindowPos(
            doc.window.0,
            None,
            240 + index as i32 * 400,
            180,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    }
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    // Layered endpoints remain click-through until first actual paint, matching ordinary-pump input prerequisites after initial display.
    for hwnd in app.chrome.handles() {
        UpdateWindow(hwnd).ok().unwrap();
    }
    assert_eq!(app.chrome.ui.nodes.borrow().len(), 2);
    assert_eq!(app.chrome.ui.input_windows.borrow().len(), 4);
    app
}
unsafe fn border_handle(ui: &Ui, endpoint: usize) -> HWND {
    let node = ui.node(endpoint).unwrap();
    ui.borders
        .borrow()
        .iter()
        .find(|border| border.matches(&node))
        .unwrap()
        .hwnd
}
unsafe fn border_covers(ui: &Ui, endpoint: usize, point: Point) -> bool {
    let hwnd = border_handle(ui, endpoint);
    let mut bounds = RECT::default();
    GetWindowRect(hwnd, &mut bounds).unwrap();
    let region = CreateRectRgn(0, 0, 0, 0);
    assert_ne!(GetWindowRgn(hwnd, region), RGN_ERROR);
    let covered = PtInRegion(region, point.x - bounds.left, point.y - bounds.top).as_bool();
    let _ = DeleteObject(region.into());
    IsWindowVisible(hwnd).as_bool() && covered
}
unsafe fn mouse(ui: &Ui, hwnd: HWND, msg: u32, point: Point) {
    let mut client = POINT {
        x: point.x,
        y: point.y,
    };
    ScreenToClient(hwnd, &mut client).ok().unwrap();
    assert!(i16::try_from(client.x).is_ok() && i16::try_from(client.y).is_ok());
    input::mouse(
        hwnd,
        ui,
        msg,
        LPARAM(((client.y as u16 as u32) << 16 | client.x as u16 as u32) as isize),
    );
}
fn port(ui: &Ui, node: usize, side: Side) -> (HWND, Port, Point) {
    let hwnd = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, endpoint, s)| *endpoint == node && *s == side)
        .unwrap()
        .0;
    let port = Port {
        node,
        side,
        index: 0,
    };
    let mut point = ui.point(port).unwrap();
    let node = ui.node(node).unwrap();
    // Hit samples must lie outside new white-border clipping; small-image radius/2 may fall exactly inside the border.
    let offset = (node.radius() / 2).max(desktop::pin_border_style::width(node.dpi) as i32 + 1);
    point.x += if side == Side::Input { -offset } else { offset };
    (hwnd, port, point)
}
#[test]
fn owned_sparse_regions_affinity_and_three_modes_keep_only_declared_input_areas() {
    unsafe {
        let mut app = visible_app();
        let ui = &app.chrome.ui;
        let mut title = [0; 64];
        let length = GetWindowTextW(ui.drawing.get(), &mut title) as usize;
        assert_eq!(
            String::from_utf16_lossy(&title[..length]),
            crate::ui_text::text("workspace-title")
        );
        for (hwnd, endpoint, side) in ui.input_windows.borrow().iter() {
            let length = GetWindowTextW(*hwnd, &mut title) as usize;
            assert_eq!(
                String::from_utf16_lossy(&title[..length]),
                crate::ui_text::text("workspace-ports-title")
            );
            let node = ui.node(*endpoint).unwrap();
            assert!(GetWindow(*hwnd, GW_OWNER).unwrap_or_default().is_invalid());
            let mut affinity = 0;
            GetWindowDisplayAffinity(*hwnd, &mut affinity).unwrap();
            assert_eq!(
                affinity,
                if node.protected {
                    WDA_EXCLUDEFROMCAPTURE.0
                } else {
                    0
                }
            );
            let (_, _, point) = port(ui, *endpoint, *side);
            assert!(occlusion::contains(*hwnd, point));
        }
        let style = GetWindowLongPtrW(ui.drawing.get(), GWL_EXSTYLE) as u32;
        assert_eq!(style & WS_EX_TRANSPARENT.0, WS_EX_TRANSPARENT.0);
        for mode in [Mode::Border, Mode::None, Mode::All] {
            ui.mode.set(mode);
            app.chrome.windows.as_ref().unwrap().refresh().unwrap();
            for (hwnd, _, _) in ui.input_windows.borrow().iter() {
                assert_eq!(IsWindowVisible(*hwnd).as_bool(), mode == Mode::All);
            }
        }
        app.shutdown();
    }
}
#[test]
fn geometry_moves_without_application_polling_and_same_catalog_reuses_the_graph() {
    unsafe {
        let mut app = visible_app();
        let first = app.chrome.ui.scene.borrow().clone().unwrap();
        for _ in 0..100 {
            app.sync_workspace_chrome(true);
            assert!(Rc::ptr_eq(
                &first,
                app.chrome.ui.scene.borrow().as_ref().unwrap()
            ));
        }
        let hwnd = app.documents[0].window.0;
        SetWindowPos(
            hwnd,
            None,
            320,
            300,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert_eq!(
            app.chrome.ui.node(0).unwrap().rect,
            scene::frame(hwnd).unwrap()
        );
        let (strip, _, point) = port(&app.chrome.ui, 0, Side::Output);
        let mut bounds = RECT::default();
        GetWindowRect(strip, &mut bounds).unwrap();
        assert!(Rect::from(bounds).contains(point));
        app.shutdown();
    }
}
#[test]
fn host_moves_paint_current_frame_and_ports_before_returning_without_a_message_pump() {
    let _apartment = desktop::Apartment::new().unwrap();
    unsafe {
        let mut app = visible_app();
        // WS_EX_TRANSPARENT waits for underlying same-thread windows' initial painting; actual dragging begins from displayed pins.
        for doc in &app.documents {
            let _ = UpdateWindow(doc.window.0);
            assert!(!GetUpdateRect(doc.window.0, None, false).as_bool());
        }
        let ui = app.chrome.ui.clone();
        let host = app.documents[0].window.0;
        let other = ui.node(1).unwrap().rect;
        for mode in [Mode::All, Mode::Border] {
            ui.mode.set(mode);
            for (x, y) in [(320, 300), (180, 220), (400, 160)] {
                SetWindowPos(
                    host,
                    None,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
                let frame = scene::frame(host).unwrap();
                assert_eq!(ui.node(0).unwrap().rect, frame);
                assert_eq!(ui.node(1).unwrap().rect, other);
                // Inspect committed border pixels directly rather than treating idle WM_PAINT counts as presentation credentials.
                assert!(ui.failure.borrow().is_none());
                for (x, y) in [(frame.left - 1, frame.top - 1), (frame.right, frame.bottom)] {
                    assert!(border_covers(&ui, 0, Point { x, y }));
                }
                for hwnd in ui
                    .handles()
                    .into_iter()
                    .filter(|h| IsWindowVisible(*h).as_bool())
                {
                    assert!(
                        !GetUpdateRect(hwnd, None, false).as_bool(),
                        "visible chrome must not defer paint"
                    );
                }
            }
        }
        ui.mode.set(Mode::None);
        window::refresh(&ui).unwrap();
        assert!(!ui.node(0).unwrap().frame(Mode::None));
        app.capture_active.set(true);
        SetWindowPos(
            host,
            None,
            360,
            200,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert!(IsWindowVisible(ui.drawing.get()).as_bool());
        assert!(ui.redraw_after_capture.get());
        app.shutdown();
    }
}
#[test]
fn click_discloses_and_drag_cancel_retains_topology_and_expands_preview_bounds() {
    unsafe {
        let mut app = visible_app();
        let ui = &app.chrome.ui;
        let (hwnd, source, point) = port(ui, 0, Side::Output);
        mouse(ui, hwnd, WM_LBUTTONDOWN, point);
        assert!(!ui.idle());
        mouse(ui, hwnd, WM_LBUTTONUP, point);
        assert_eq!(ui.disclosed.borrow().get(&source), Some(&true));
        // Expanded anchors have moved outward; old coordinates are body, not a valid basis for fabricated connection drags.
        let (_, _, point) = port(ui, 0, Side::Output);
        mouse(ui, hwnd, WM_LBUTTONDOWN, point);
        let far = Point {
            x: point.x + 600,
            y: point.y + 250,
        };
        mouse(ui, hwnd, WM_MOUSEMOVE, far);
        assert!(ui.press.borrow().as_ref().unwrap().dragged);
        let mut bounds = RECT::default();
        GetWindowRect(ui.drawing.get(), &mut bounds).unwrap();
        assert!(Rect::from(bounds).contains(far));
        ui.cancel();
        assert!(ui.idle());
        assert_ne!(GetCapture(), hwnd);
        assert!(app.workspace.links().is_empty());
        app.shutdown();
    }
}
#[test]
fn actual_drop_queues_once_and_reconnect_cancel_preserves_original_edge() {
    unsafe {
        let mut app = visible_app();
        // Use actual borderless pin hosts; system nonclient areas must not become extra resize borders above endpoints.
        for doc in &app.documents {
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
        }
        let ui = &app.chrome.ui;
        let (source_hwnd, _, source) = port(ui, 0, Side::Output);
        let (target_hwnd, _, target) = port(ui, 1, Side::Input);
        assert_eq!(
            WindowFromPoint(POINT {
                x: target.x,
                y: target.y
            }),
            target_hwnd
        );
        mouse(ui, source_hwnd, WM_LBUTTONDOWN, source);
        mouse(ui, source_hwnd, WM_MOUSEMOVE, target);
        mouse(ui, source_hwnd, WM_LBUTTONUP, target);
        assert!(ui.pending.borrow().is_some());
        assert!(!app.chrome.idle());
        app.pending_link_changes();
        assert_eq!(app.workspace.links().len(), 1);
        assert!(app.chrome.idle());
        assert_eq!(
            GetClassLongPtrW(source_hwnd, GCLP_HCURSOR),
            LoadCursorW(None, IDC_ARROW).unwrap().0 as usize
        );
        assert_eq!(
            GetClassLongPtrW(target_hwnd, GCLP_HCURSOR),
            LoadCursorW(None, IDC_ARROW).unwrap().0 as usize
        );
        app.sync_workspace_chrome(true);
        let old = app.workspace.links()[0].clone();
        let ui = &app.chrome.ui;
        mouse(ui, target_hwnd, WM_LBUTTONDOWN, target);
        mouse(
            ui,
            target_hwnd,
            WM_MOUSEMOVE,
            Point {
                x: target.x,
                y: target.y + 200,
            },
        );
        assert!(ui.press.borrow().as_ref().unwrap().source.is_some());
        ui.cancel();
        app.pending_link_changes();
        assert_eq!(app.workspace.links(), [old]);
        app.shutdown();
    }
}
#[test]
fn capture_retires_gestures_without_hiding_and_host_destruction_retires_every_chrome_hwnd() {
    unsafe {
        let mut app = visible_app();
        let (hwnd, _, point) = port(&app.chrome.ui, 0, Side::Output);
        mouse(&app.chrome.ui, hwnd, WM_LBUTTONDOWN, point);
        let visibility: Vec<_> = app
            .chrome
            .handles()
            .into_iter()
            .map(|hwnd| (hwnd, IsWindowVisible(hwnd).as_bool()))
            .collect();
        app.capture_active.set(true);
        app.sync_workspace_chrome(true);
        assert!(app.chrome.idle());
        assert!(app.chrome.ui.blocked());
        // Backdrops without consecutively expanded rows stay hidden; screenshots neither hide previously visible windows nor show previously hidden ones.
        for (hwnd, visible) in visibility {
            assert_eq!(IsWindowVisible(hwnd).as_bool(), visible);
        }
        app.capture_active.set(false);
        app.sync_workspace_chrome(true);
        let handles = app.chrome.handles();
        DestroyWindow(app.documents[0].window.0).unwrap();
        assert!(app.chrome.ui.node(0).is_none());
        app.shutdown();
        assert!(handles.iter().all(|h| !IsWindow(Some(*h)).as_bool()));
    }
}
#[test]
fn first_scene_while_target_hidden_keeps_links_for_same_catalog_restore() {
    unsafe {
        let mut app = visible_app();
        let catalog = app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .unwrap()
            .catalog
            .clone();
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation { catalog, link });
        app.documents[1].state.group_hidden.set(true);
        let _ = ShowWindow(app.documents[1].window.0, SW_HIDE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let scene = app.chrome.ui.scene.borrow().clone().unwrap();
        assert_eq!(scene.edges.len(), 1);
        assert_eq!(app.chrome.ui.nodes.borrow().len(), 1);
        app.documents[1].state.group_hidden.set(false);
        let _ = ShowWindow(app.documents[1].window.0, SW_SHOWNOACTIVATE);
        app.sync_workspace_chrome(true);
        assert!(Rc::ptr_eq(
            &scene,
            app.chrome.ui.scene.borrow().as_ref().unwrap()
        ));
        assert_eq!(app.chrome.ui.nodes.borrow().len(), 2);
        app.shutdown();
    }
}
struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(held: &[VIRTUAL_KEY]) -> Self {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let mut keys = [0; 256];
        for key in held {
            keys[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        Self(original)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}
#[test]
fn exact_thread_local_chord_cycles_once_repeat_and_extra_modifiers_do_not_toggle() {
    unsafe {
        let mut app = visible_app();
        let mut message = MSG {
            hwnd: app.documents[0].window.0,
            message: WM_KEYDOWN,
            wParam: WPARAM(0x34),
            ..Default::default()
        };
        let _keys = Keys::held(&[VK_CONTROL, VK_SHIFT]);
        for expected in [Mode::Border, Mode::None, Mode::All] {
            message.lParam = LPARAM(0);
            assert!(app.translate_workspace_chrome(&message));
            assert_eq!(app.chrome.ui.mode.get(), expected);
            message.lParam = LPARAM(1 << 30);
            assert!(app.translate_workspace_chrome(&message));
            assert_eq!(app.chrome.ui.mode.get(), expected);
        }
        for extra in [VK_MENU, VK_LWIN, VK_RWIN] {
            let _extra = Keys::held(&[VK_CONTROL, VK_SHIFT, extra]);
            message.lParam = LPARAM(0);
            assert!(!app.translate_workspace_chrome(&message));
            assert_eq!(app.chrome.ui.mode.get(), Mode::All);
        }
        app.shutdown();
    }
}
