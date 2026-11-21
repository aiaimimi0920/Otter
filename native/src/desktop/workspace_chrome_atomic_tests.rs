//! 共享外框不得先搬动旧画面再重绘；仅操作隔离 desktop 的自有窗口。
use super::*;
struct Probe {
    hwnd: HWND,
    stale_transactions: Cell<u32>,
    positions: Cell<u32>,
    zorders: Cell<u32>,
}
impl Probe {
    unsafe fn new(hwnd: HWND) -> Box<Self> {
        let owner = Box::new(Self {
            hwnd,
            stale_transactions: Cell::new(0),
            positions: Cell::new(0),
            zorders: Cell::new(0),
        });
        SetWindowSubclass(hwnd, Some(observe), 548, &*owner as *const _ as usize).unwrap();
        owner
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        unsafe { RemoveWindowSubclass(self.hwnd, Some(observe), 548).unwrap() };
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const Probe);
    if msg == WM_WINDOWPOSCHANGED {
        probe.positions.set(probe.positions.get() + 1);
    }
    if msg == WM_WINDOWPOSCHANGING && IsWindowVisible(hwnd).as_bool() {
        let position = &*(lp.0 as *const WINDOWPOS);
        if !position.flags.contains(SWP_NOZORDER) {
            probe.zorders.set(probe.zorders.get() + 1);
        }
        if !position.flags.contains(SWP_NOMOVE) || !position.flags.contains(SWP_NOSIZE) {
            let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS::default();
            // SetLayeredWindowAttributes 的重定向内容会先被整张移位；ULW 随新像素一并定位。
            if GetLayeredWindowAttributes(hwnd, None, None, Some(&mut flags)).is_ok() {
                probe
                    .stale_transactions
                    .set(probe.stale_transactions.get() + 1);
            }
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
unsafe fn stationary_frame_is_white(ui: &Ui, endpoint: usize) {
    let node = ui.node(endpoint).unwrap();
    assert!(border_covers(
        ui,
        endpoint,
        Point {
            x: node.rect.left - 1,
            y: node.rect.top - 1
        }
    ));
    assert_eq!(
        ui.borders
            .borrow()
            .iter()
            .find(|b| b.hwnd == border_handle(ui, endpoint))
            .unwrap()
            .alpha(),
        255
    );
}
pub(super) unsafe fn links_below_nodes(ui: &Ui) {
    let layers = [ui.drawing.get()];
    for layer in layers {
        let mut above = HashSet::new();
        let mut cursor = GetWindow(layer, GW_HWNDPREV).unwrap_or_default();
        for _ in 0..4096 {
            if cursor.is_invalid() || !above.insert(cursor.0 as isize) {
                break;
            }
            cursor = GetWindow(cursor, GW_HWNDPREV).unwrap_or_default();
        }
        for node in ui.nodes.borrow().iter() {
            assert!(
                above.contains(&node.hwnd),
                "shared curves must be below every pin: {layer:?} / {}",
                node.endpoint
            );
            let border = border_handle(ui, node.endpoint);
            if IsWindowVisible(border).as_bool() {
                assert!(
                    above.contains(&(border.0 as isize)),
                    "complete preview/border must cover the whole connection layer"
                );
            }
        }
    }
    for node in ui.nodes.borrow().iter() {
        let host = HWND(node.hwnd as *mut _);
        let mut previous = host;
        for hwnd in ui
            .input_windows
            .borrow()
            .iter()
            .filter(|(_, endpoint, _)| *endpoint == node.endpoint)
            .map(|(hwnd, _, _)| *hwnd)
            .chain(drawer::moving(ui, host))
            .chain(
                ui.backings
                    .borrow()
                    .iter()
                    .filter(|b| b.matches(node))
                    .map(|b| b.hwnd),
            )
        {
            assert_eq!(
                GetWindow(previous, GW_HWNDNEXT).unwrap(),
                hwnd,
                "Tab members must be adjacent to their own pin without interleaving"
            );
            assert_eq!(
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                GetWindowLongPtrW(host, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0
            );
            previous = hwnd;
        }
    }
}
#[test]
fn moving_and_resizing_shared_chrome_never_repositions_an_old_redirected_frame() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let probe = Probe::new(ui.drawing.get());
        for (x, y, width, height) in [
            (300, 220, 160, 120),
            (280, 160, 200, 150),
            (240, 180, 160, 120),
        ] {
            SetWindowPos(
                app.documents[0].window.0,
                None,
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            assert!(IsWindowVisible(ui.drawing.get()).as_bool());
            stationary_frame_is_white(&ui, 1);
        }
        assert_eq!(
            probe.stale_transactions.get(),
            0,
            "all stationary borders must stay at their screen coordinates"
        );
        assert_eq!(
            probe.zorders.get(),
            0,
            "geometry refresh must not raise shared chrome"
        );
        drop(probe);
        app.shutdown();
    }
}
#[test]
fn shared_frames_respect_overlapping_pin_content_and_keep_upper_border() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        SetWindowPos(
            app.documents[0].window.0,
            None,
            200,
            200,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        SetWindowPos(
            app.documents[1].window.0,
            Some(HWND_TOPMOST),
            300,
            180,
            160,
            120,
            SWP_NOACTIVATE,
        )
        .unwrap();
        app.sync_workspace_chrome(true);
        let lower = ui.node(0).unwrap().rect;
        let upper = ui.node(1).unwrap().rect;
        let y = lower.top.max(upper.top) + 10;
        assert!(upper.left < lower.right && lower.right < upper.right && y < upper.bottom);
        assert!(lower.left < upper.left - 1 && upper.left - 1 < lower.right && y < lower.bottom);
        let upper_host = app.documents[1].window.0;
        let lower_host = app.documents[0].window.0;
        let upper_border = border_handle(&ui, 1);
        let lower_border = border_handle(&ui, 0);
        assert_eq!(GetWindow(upper_border, GW_HWNDNEXT).unwrap(), upper_host);
        assert_eq!(GetWindow(lower_border, GW_HWNDNEXT).unwrap(), lower_host);
        let mut cursor = GetWindow(upper_host, GW_HWNDNEXT).unwrap_or_default();
        for _ in 0..128 {
            if cursor == lower_border || cursor.is_invalid() {
                break;
            }
            cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
        }
        assert_eq!(
            cursor, lower_border,
            "lower border must remain below upper image"
        );
        assert!(
            border_covers(
                &ui,
                1,
                Point {
                    x: upper.left - 1,
                    y
                }
            ),
            "upper border overlaps lower image"
        );
        app.shutdown();
    }
}
#[test]
fn capture_resume_and_new_node_publish_complete_stationary_frames_without_repositioning_old_pixels()
{
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let probe = Probe::new(ui.drawing.get());
        let positions = probe.positions.get();
        let host = app.documents[0].window.0;
        geometry(&app.documents[0].state.chrome, host, WM_WINDOWPOSCHANGED);
        assert_eq!(
            probe.positions.get(),
            positions,
            "unchanged host geometry must not resubmit the shared layer"
        );
        app.capture_active.set(true);
        app.sync_workspace_chrome(true);
        stationary_frame_is_white(&ui, 1);
        app.capture_active.set(false);
        app.add_window(false).unwrap();
        let host = app.documents.last().unwrap().window.0;
        SetWindowPos(
            host,
            None,
            900,
            560,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        links_below_nodes(&ui);
        stationary_frame_is_white(&ui, 1);
        assert_eq!(probe.stale_transactions.get(), 0);
        assert!(IsWindowVisible(ui.drawing.get()).as_bool());
        drop(probe);
        app.shutdown();
        assert!(
            ui.surface.borrow().is_none(),
            "closing the chrome owner releases its bounded bitmap"
        );
    }
}
#[test]
fn connected_and_preview_links_stay_below_unlinked_pins_in_every_mode() {
    unsafe {
        let mut app = visible_app();
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation { catalog, link });
        app.add_window(false).unwrap();
        let _ = ShowWindow(app.documents[2].window.0, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let ui = app.chrome.ui.clone();
        assert_eq!(ui.scene.borrow().as_ref().unwrap().edges.len(), 1);
        assert_eq!(ui.nodes.borrow().len(), 3);
        let (strip, _, start) = port(&ui, 0, Side::Output);
        mouse(&ui, strip, WM_LBUTTONDOWN, start);
        mouse(
            &ui,
            strip,
            WM_MOUSEMOVE,
            Point {
                x: start.x + 60,
                y: start.y + 30,
            },
        );
        assert!(ui.press.borrow().as_ref().unwrap().dragged);
        for mode in [Mode::All, Mode::Border, Mode::None] {
            ui.mode.set(mode);
            app.chrome.windows.as_ref().unwrap().refresh().unwrap();
            links_below_nodes(&ui);
        }
        app.shutdown();
    }
}
#[test]
fn pure_pin_reordering_lowers_only_links_before_return_without_a_geometry_refresh() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let probe = Probe::new(ui.drawing.get());
        let surface = ui.surface.borrow().as_ref().unwrap().dc();
        links_below_nodes(&ui);
        for index in [0, 1, 0, 1] {
            let host = app.documents[index].window.0;
            let before = ui.node(index).unwrap().rect;
            // 人为把自有宿主放到线层后；同步位置通知必须立即纠正，不能等应用轮询。
            SetWindowPos(
                host,
                Some(ui.drawing.get()),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .unwrap();
            links_below_nodes(&ui);
            assert_eq!(ui.node(index).unwrap().rect, before);
            assert_eq!(
                GetWindow(border_handle(&ui, index), GW_HWNDNEXT).unwrap(),
                host
            );
            assert_eq!(ui.surface.borrow().as_ref().unwrap().dc(), surface);
        }
        let repairs = probe.zorders.get();
        assert_eq!(repairs, 4);
        app.chrome.windows.as_ref().unwrap().refresh().unwrap();
        assert_eq!(
            probe.zorders.get(),
            repairs,
            "unchanged refresh must not reorder links"
        );
        drop(probe);
        app.shutdown();
    }
}
