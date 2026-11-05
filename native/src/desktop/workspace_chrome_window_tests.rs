//! 自有 HWND 的 region/捕获/几何/拓扑与资源门；由隔离 desktop runner 执行。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};
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
    // layered端点在首个真实paint前仍穿透；与普通消息泵完成首显后的输入前提一致。
    for hwnd in app.chrome.handles() {
        UpdateWindow(hwnd).ok().unwrap();
    }
    assert_eq!(app.chrome.ui.nodes.borrow().len(), 2);
    assert_eq!(app.chrome.ui.input_windows.borrow().len(), 4);
    app
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
    (hwnd, port, ui.point(port).unwrap())
}
#[test]
fn owned_sparse_regions_affinity_and_three_modes_keep_only_declared_input_areas() {
    unsafe {
        let mut app = visible_app();
        let ui = &app.chrome.ui;
        let mut title = [0; 64];
        let length = GetWindowTextW(ui.drawing.get(), &mut title) as usize;
        assert_eq!(String::from_utf16_lossy(&title[..length]), "Otter · 工作区");
        for (hwnd, endpoint, _) in ui.input_windows.borrow().iter() {
            let length = GetWindowTextW(*hwnd, &mut title) as usize;
            assert_eq!(String::from_utf16_lossy(&title[..length]), "Otter · 端口");
            let node = ui.node(*endpoint).unwrap();
            assert_eq!(
                GetWindow(*hwnd, GW_OWNER).unwrap(),
                HWND(node.hwnd as *mut _)
            );
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
            let region = CreateRectRgn(0, 0, 0, 0);
            assert_ne!(GetWindowRgn(*hwnd, region), RGN_ERROR);
            assert!(PtInRegion(region, 12, 12).as_bool());
            let _ = DeleteObject(HGDIOBJ(region.0));
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
fn click_discloses_and_drag_cancel_retains_topology_and_expands_preview_bounds() {
    unsafe {
        let mut app = visible_app();
        let ui = &app.chrome.ui;
        let (hwnd, source, point) = port(ui, 0, Side::Output);
        mouse(ui, hwnd, WM_LBUTTONDOWN, point);
        assert!(!ui.idle());
        mouse(ui, hwnd, WM_LBUTTONUP, point);
        assert!(ui.disclosed.borrow().contains(&source));
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
fn capture_pause_and_host_destruction_retire_gestures_and_every_chrome_hwnd() {
    unsafe {
        let mut app = visible_app();
        let (hwnd, _, point) = port(&app.chrome.ui, 0, Side::Output);
        mouse(&app.chrome.ui, hwnd, WM_LBUTTONDOWN, point);
        app.capture_active.set(true);
        app.sync_workspace_chrome(true);
        assert!(app.chrome.idle());
        assert!(app
            .chrome
            .handles()
            .iter()
            .all(|h| !IsWindowVisible(*h).as_bool()));
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
