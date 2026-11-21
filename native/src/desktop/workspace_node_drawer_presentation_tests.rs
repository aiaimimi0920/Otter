//! 底栏逐钉图层序、展开中间几何和端点共存；只观察自有 HWND，不操作用户桌面。
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

unsafe fn rank(hwnd: HWND) -> usize {
    let mut cursor = GetTopWindow(None).unwrap();
    for index in 0..4096 {
        if cursor == hwnd {
            return index;
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
        if cursor.is_invalid() {
            break;
        }
    }
    panic!("missing own window: {hwnd:?}");
}

#[test]
fn drawer_caret_and_body_follow_own_pin_across_foreign_window_and_topmost_bands() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let lower = app.documents.last().unwrap().window.0;
        let _ = ShowWindow(lower, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        let open = select(&mut app);
        let upper = app.documents[0].window.0;
        let foreign = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("independent window"),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            40,
            40,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
        SetWindowPos(lower, Some(HWND_BOTTOM), 0, 0, 0, 0, flags).unwrap();
        SetWindowPos(foreign, Some(HWND_TOP), 0, 0, 0, 0, flags).unwrap();
        for band in [HWND_TOPMOST, HWND_NOTOPMOST, HWND_TOPMOST] {
            SetWindowPos(upper, Some(band), 0, 0, 0, 0, flags).unwrap();
            for expanded in [false, true, false] {
                if open.data.open.get() != expanded {
                    click(&open, DRAWER_TOGGLE);
                    app.sync_workspace_chrome(true);
                }
                assert!(rank(upper) < rank(foreign) && rank(foreign) < rank(lower));
                assert!(rank(upper) < rank(open.window.0));
                assert!(
                    rank(open.window.0) < rank(foreign),
                    "drawer must follow its own pin"
                );
                assert_eq!(
                    GetWindowLongPtrW(open.window.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                    GetWindowLongPtrW(upper, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                );
                let button = GetDlgItem(Some(open.drawer.handle()), DRAWER_TOGGLE as i32).unwrap();
                let _ = SetFocus(Some(button));
                assert!(rank(upper) < rank(open.window.0) && rank(open.window.0) < rank(foreign));
            }
        }
        DestroyWindow(foreign).unwrap();
        app.shutdown();
    }
}

struct Positions {
    caret: HWND,
    values: RefCell<Vec<RECT>>,
}
struct Watch {
    positions: Box<Positions>,
    handles: [HWND; 3],
}
impl Drop for Watch {
    fn drop(&mut self) {
        for hwnd in self.handles {
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(observe), 613);
            }
        }
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    pointer: usize,
) -> LRESULT {
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if matches!(msg, WM_WINDOWPOSCHANGED | WM_PAINT) {
        let data = &*(pointer as *const Positions);
        let mut rect = RECT::default();
        if GetWindowRect(data.caret, &mut rect).is_ok() {
            data.values.borrow_mut().push(rect);
        }
    }
    result
}

#[test]
fn drawer_toggle_never_publishes_a_horizontal_caret_detour() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        let caret = GetDlgItem(Some(open.drawer.handle()), DRAWER_TOGGLE as i32).unwrap();
        let mut initial = RECT::default();
        GetWindowRect(caret, &mut initial).unwrap();
        let watch = Watch {
            positions: Box::new(Positions {
                caret,
                values: RefCell::default(),
            }),
            handles: [open.window.0, open.drawer.handle(), caret],
        };
        for hwnd in watch.handles {
            assert!(SetWindowSubclass(
                hwnd,
                Some(observe),
                613,
                &*watch.positions as *const _ as usize
            )
            .as_bool());
        }
        for _ in 0..4 {
            for expanded in [true, false] {
                watch.positions.values.borrow_mut().clear();
                click(&open, DRAWER_TOGGLE);
                app.sync_workspace_chrome(true);
                assert_eq!(open.data.open.get(), expanded);
                let samples = watch.positions.values.borrow();
                assert!(!samples.is_empty());
                assert!(
                    samples
                        .iter()
                        .all(|r| r.left == initial.left && r.right == initial.right),
                    "caret moved sideways during disclosure: {samples:?}"
                );
                let last = samples.last().unwrap();
                assert_eq!(last.top > initial.top, expanded);
                assert!(!GetUpdateRect(open.drawer.handle(), None, false).as_bool());
            }
        }
        drop(watch);
        app.shutdown();
    }
}

pub(super) unsafe fn endpoint_click(ui: &Ui, target: Port) {
    let row = port_visual::layout(ui, target).unwrap();
    let (x, y) = row.connection();
    let hwnd = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == target.node && *s == target.side)
        .unwrap()
        .0;
    let mut p = POINT {
        x: x + if target.side == Side::Input { -8 } else { 8 },
        y,
    };
    ScreenToClient(hwnd, &mut p).ok().unwrap();
    let lp = LPARAM(p.x as u16 as isize | ((p.y as u16 as isize) << 16));
    SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(lp));
    SendMessageW(hwnd, WM_LBUTTONUP, None, Some(lp));
}

#[test]
fn selection_switch_still_retires_a_drawer_opened_without_total_tab() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let host = open.data.host;
        app.add_window(false).unwrap();
        let other = &app.documents[1];
        let _ = ShowWindow(other.window.0, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        selection::observe(
            &app.documents[1].state.chrome,
            app.documents[1].window.0,
            WM_LBUTTONDOWN,
        );
        app.sync_workspace_chrome(true);
        assert!(!app
            .chrome
            .ui
            .drawers
            .borrow()
            .iter()
            .any(|o| o.data.host == host));
        drop(open);
        app.shutdown();
    }
}

#[test]
fn endpoint_clicks_and_bottom_drawer_coexist_without_touching_other_pins() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let _ = ShowWindow(app.documents.last().unwrap().window.0, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        let open = select(&mut app);
        let ui = app.chrome.ui.clone();
        let other = Port {
            node: 1,
            side: Side::Output,
            index: 0,
        };
        endpoint_click(&ui, other);
        let target = Port {
            node: 0,
            side: Side::Output,
            index: 0,
        };
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        for side in [Side::Input, Side::Output] {
            assert!(disclosure::expanded(&ui, other, &ui.node(1).unwrap()));
            assert!(open.data.open.get());
            let target = Port { side, ..target };
            endpoint_click(&ui, target);
            assert!(
                open.data.open.get(),
                "endpoint expansion must not collapse the bottom drawer"
            );
            assert!(IsWindowVisible(GetDlgItem(Some(open.drawer.handle()), 5).unwrap()).as_bool());
            assert!(disclosure::expanded(&ui, target, &ui.node(0).unwrap()));
            app.sync_workspace_chrome(true);
        }
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        assert!(!open.data.open.get());
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        assert!(open.data.open.get());
        for side in [Side::Input, Side::Output] {
            assert!(disclosure::expanded(
                &ui,
                Port { side, ..target },
                &ui.node(0).unwrap()
            ));
        }
        assert!(disclosure::expanded(&ui, other, &ui.node(1).unwrap()));
        app.shutdown();
    }
}

#[test]
fn parameter_focus_preserves_drafts_and_the_open_bottom_drawer() {
    unsafe {
        let mut app = app();
        art(&mut app);
        let open = select(&mut app);
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        let edit = text_edit(app.documents[0].state.parameters_ui.window(), "原值");
        SetWindowTextW(edit, w!("保留草稿")).unwrap();
        assert!(open.data.open.get());
        let ui = app.chrome.ui.clone();
        let index = ui.scene.borrow().as_ref().unwrap().catalog.endpoints[0]
            .inputs
            .len()
            - 1;
        assert!(parameter_rail::request(
            &ui,
            Port {
                node: 0,
                side: Side::Input,
                index
            },
            true
        ));
        assert!(open.data.open.get());
        assert!(open.data.pending.get().is_none());
        app.sync_workspace_chrome(true);
        assert_eq!(
            text_edit(app.documents[0].state.parameters_ui.window(), "保留草稿"),
            edit
        );
        assert!(IsWindowVisible(edit).as_bool());
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.shutdown();
    }
}

#[test]
fn total_tab_opens_both_endpoints_and_drawer_when_collapsed_endpoints_are_hidden() {
    unsafe {
        let mut app = app();
        let state = &app.documents[0].state;
        state
            .node_settings
            .set(crate::unit_node_settings::Settings {
                hide_collapsed_endpoints: true,
                ..state.node_settings.get()
            });
        let open = select(&mut app);
        assert!(!IsWindowVisible(open.window.0).as_bool());
        for _ in 0..2 {
            app.documents[0].state.parameters_ui.toggle.set(true);
            app.process_parameter_panels(true);
            app.sync_workspace_chrome(true);
            assert!(open.data.open.get());
            assert!(IsWindowVisible(open.window.0).as_bool());
            assert!(app.documents[0].state.parameters_ui.expanded.get());
            click(&open, DRAWER_TOGGLE);
            app.sync_workspace_chrome(true);
            assert!(!IsWindowVisible(open.window.0).as_bool());
            assert!(app.documents[0].state.parameters_ui.expanded.get());
            app.documents[0].state.parameters_ui.toggle.set(true);
            app.process_parameter_panels(true);
            app.sync_workspace_chrome(true);
        }
        app.shutdown();
    }
}
