//! Owned HWNDs observe repeated region notifications, capture visibility, and zoom-paint timing on isolated desktops only.
use super::*;
struct Probe {
    hwnd: HWND,
    initially_visible: bool,
    positions: Cell<u32>,
    visibility: Cell<u32>,
    paints: Cell<u32>,
    stale_viewport: Cell<bool>,
    check_viewport: bool,
    missing_zoom_frame: Cell<bool>,
    zoom_frames: Cell<u32>,
    stale_move_frame: Cell<bool>,
}
impl Probe {
    unsafe fn new(hwnd: HWND, check_viewport: bool) -> Box<Self> {
        let owner = Box::new(Self {
            hwnd,
            initially_visible: IsWindowVisible(hwnd).as_bool(),
            positions: Cell::new(0),
            visibility: Cell::new(0),
            paints: Cell::new(0),
            stale_viewport: Cell::new(false),
            check_viewport,
            missing_zoom_frame: Cell::new(false),
            zoom_frames: Cell::new(0),
            stale_move_frame: Cell::new(false),
        });
        SetWindowSubclass(hwnd, Some(observe), 547, &*owner as *const _ as usize).unwrap();
        owner
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        unsafe { RemoveWindowSubclass(self.hwnd, Some(observe), 547).unwrap() };
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
        if probe.check_viewport {
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const desktop::State);
            let pos = &*(lp.0 as *const WINDOWPOS);
            if !pos.flags.contains(SWP_NOMOVE) {
                if let Some(ui) = state.chrome.borrow().upgrade() {
                    let nodes = ui.nodes.borrow();
                    if let Some(node) = nodes.iter().find(|n| n.hwnd == hwnd.0 as isize) {
                        probe.stale_move_frame.set(
                            probe.stale_move_frame.get()
                                || node.rect.left != pos.x
                                || node.rect.top != pos.y,
                        );
                    }
                }
            }
        }
    }
    if msg == WM_WINDOWPOSCHANGING && probe.check_viewport {
        let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const desktop::State);
        let pos = &*(lp.0 as *const WINDOWPOS);
        if let Some(target) = state
            .pin_resize
            .rect
            .get()
            .filter(|r| pos.cx == r.width as i32 && pos.cy == r.height as i32)
        {
            if let Some(ui) = state.chrome.borrow().upgrade() {
                let covered = if let Some(proxy) = desktop::pin_window::preview::handle(state) {
                    let mut bounds = RECT::default();
                    let valid = GetWindowRect(proxy, &mut bounds).is_ok();
                    let padding = desktop::pin_border_style::width(
                        windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                    ) as i32;
                    let region = CreateRectRgn(0, 0, 0, 0);
                    let empty = GetWindowRgn(hwnd, region) == NULLREGION;
                    let _ = DeleteObject(region.into());
                    empty
                        && IsWindowVisible(proxy).as_bool()
                        && valid
                        && bounds.left == target.x - padding
                        && bounds.top == target.y - padding
                        && bounds.right == target.x + target.width as i32 + padding
                        && bounds.bottom == target.y + target.height as i32 + padding
                } else {
                    let node = ui
                        .nodes
                        .borrow()
                        .iter()
                        .find(|node| node.hwnd == hwnd.0 as isize)
                        .unwrap()
                        .endpoint;
                    border_covers(
                        &ui,
                        node,
                        Point {
                            x: target.x - 1,
                            y: target.y - 1,
                        },
                    )
                };
                probe.missing_zoom_frame.set(
                    probe.missing_zoom_frame.get()
                        || !covered
                        || !IsWindowVisible(ui.drawing.get()).as_bool(),
                );
                probe.zoom_frames.set(probe.zoom_frames.get() + 1);
            }
        }
    }
    if msg == WM_SHOWWINDOW {
        probe.visibility.set(probe.visibility.get() + 1);
    }
    if msg == WM_PAINT {
        probe.paints.set(probe.paints.get() + 1);
        if probe.check_viewport {
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const desktop::State);
            let mut client = RECT::default();
            if GetClientRect(hwnd, &mut client).is_ok() {
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(1) as f32;
                let canvas = state.canvas.borrow();
                let scale = client.right.min(client.bottom) as f32 * 96.0 / dpi / 8.0;
                let expected = crate::viewport::Point {
                    x: client.right as f32 * 48.0 / dpi - scale * 4.0,
                    y: client.bottom as f32 * 48.0 / dpi - scale * 4.0,
                };
                probe.stale_viewport.set(
                    probe.stale_viewport.get()
                        || (canvas.viewport.scale - scale).abs() > 0.001
                        || (canvas.viewport.origin.x - expected.x).abs() > 0.001
                        || (canvas.viewport.origin.y - expected.y).abs() > 0.001,
                );
            }
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn moving_host_prepares_border_before_position_changed_notification() {
    unsafe {
        let mut app = visible_app();
        let hwnd = app.documents[0].window.0;
        let probe = Probe::new(hwnd, true);
        for x in [260, 520, 300, 600, 280] {
            SetWindowPos(
                hwnd,
                None,
                x,
                240,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
        }
        assert!(
            !probe.stale_move_frame.get(),
            "host moved before shared border was prepared"
        );
        assert_eq!(probe.visibility.get(), 0);
        drop(probe);
        app.shutdown();
    }
}
#[test]
fn moving_one_host_does_not_reset_stationary_port_regions() {
    unsafe {
        let mut app = visible_app();
        let (strip, _, _) = port(&app.chrome.ui, 1, Side::Output);
        let probe = Probe::new(strip, false);
        for x in [260, 280, 300, 320] {
            SetWindowPos(
                app.documents[0].window.0,
                None,
                x,
                180,
                160,
                120,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
        }
        assert_eq!(
            probe.positions.get(),
            0,
            "unchanged region must not trigger a new window transaction"
        );
        assert_eq!(probe.visibility.get(), 0);
        drop(probe);
        app.shutdown();
    }
}
#[test]
fn capture_keeps_visible_chrome_frozen_and_resumes_without_hide_show() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let probes: Vec<_> = ui
            .handles()
            .into_iter()
            .map(|h| Probe::new(h, false))
            .collect();
        app.capture_active.set(true);
        app.sync_workspace_chrome(true);
        ui.repaint();
        for h in ui.handles().into_iter().rev() {
            let _ = UpdateWindow(h);
        }
        assert!(ui.blocked());
        assert!(probes
            .iter()
            .all(|p| IsWindowVisible(p.hwnd).as_bool() == p.initially_visible));
        assert!(ui.redraw_after_capture.get());
        app.capture_active.set(false);
        app.sync_workspace_chrome(true);
        assert!(!ui.redraw_after_capture.get());
        assert!(probes.iter().all(|p| p.visibility.get() == 0));
        assert!(probes
            .iter()
            .all(|p| IsWindowVisible(p.hwnd).as_bool() == p.initially_visible));
        drop(probes);
        app.shutdown();
    }
}
#[test]
fn captured_pin_delivery_keeps_previous_hosts_and_chrome_visible() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let probes: Vec<_> = ui
            .handles()
            .into_iter()
            .chain(app.documents.iter().map(|d| d.window.0))
            .map(|h| Probe::new(h, false))
            .collect();
        app.capture_active.set(true);
        app.sync_workspace_chrome(true);
        let doc = &app.documents[0];
        desktop::capture_input::run_positioned(doc.window.0, &doc.state, true, || {
            ui.repaint();
            for h in ui.handles().into_iter().rev() {
                let _ = UpdateWindow(h);
            }
            assert!(ui.blocked());
            Ok(Some((
                desktop::image::Image {
                    width: 8,
                    height: 8,
                    pixels: [30, 60, 90, 255].repeat(64).into(),
                },
                Some(POINT { x: 500, y: 360 }),
            )))
        })
        .unwrap();
        app.capture_active.set(false);
        desktop::open_job::finish(doc.window.0, &doc.state, true);
        app.pending_windows(true).unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        assert_eq!(app.documents.len(), 3);
        assert_eq!(ui.nodes.borrow().len(), 3);
        assert!(!ui.redraw_after_capture.get());
        for probe in &probes {
            assert_eq!(probe.visibility.get(), 0);
            assert_eq!(
                IsWindowVisible(probe.hwnd).as_bool(),
                probe.initially_visible
            );
        }
        drop(probes);
        app.shutdown();
    }
}
#[test]
fn bound_chrome_resize_never_paints_the_previous_viewport_in_the_new_client() {
    check_zoom_chrome(Some(desktop::render_commands::SOFTWARE));
}
#[test]
fn gpu_zoom_masks_native_geometry_until_complete_preview_and_restores_chrome() {
    check_zoom_chrome(None);
}
fn check_zoom_chrome(command: Option<usize>) {
    unsafe {
        let mut app = visible_app();
        let doc = &app.documents[0];
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas.pinned = true;
            canvas
                .replace_image(desktop::image::Image {
                    width: 8,
                    height: 8,
                    pixels: [30, 60, 90, 255].repeat(64).into(),
                })
                .unwrap();
        }
        desktop::pin_window::fit(doc.window.0, &doc.state);
        if let Some(command) = command {
            desktop::render_commands::execute(doc.window.0, &doc.state, command).unwrap();
        }
        let _ = UpdateWindow(doc.window.0);
        let probe = Probe::new(doc.window.0, true);
        let chrome_dc = app.chrome.ui.surface.borrow().as_ref().unwrap().dc();
        let confirmed = doc.state.canvas.borrow().confirmed_paints;
        let painted = doc.state.canvas.borrow().paints;
        // At double sensitivity, retain this test's premise of zooming only within existing spare space.
        for delta in [60i16, 60, -60, 15, -15, -60] {
            desktop::pin_window::wheel(
                doc.window.0,
                &doc.state,
                WPARAM((delta as u16 as usize) << 16),
                LPARAM(280 | (220 << 16)),
            );
            desktop::pin_window::animation::settle(doc.window.0, &doc.state);
            desktop::pin_window::animation::stop(doc.window.0, &doc.state);
            assert!(doc.state.error.borrow().is_none());
            assert_eq!(
                app.chrome.ui.surface.borrow().as_ref().unwrap().dc(),
                chrome_dc,
                "zoom inside reserved bounds must reuse the shared border surface"
            );
        }
        // Software scaling completes atomic ULW submission directly; do not draw again merely to increase WM_PAINT counts.
        if command.is_some() {
            assert!(doc.state.canvas.borrow().confirmed_paints > confirmed);
        } else {
            assert!(doc.state.canvas.borrow().paints > painted);
            assert!(probe.zoom_frames.get() > 0);
        }
        // Software ULW submission does not guarantee WINDOWPOSCHANGING; verify per-frame surface reuse and white borders separately.
        assert!(
            !probe.missing_zoom_frame.get(),
            "target border must exist before host geometry changes"
        );
        assert!(
            !probe.stale_viewport.get(),
            "WM_SIZE must fit the viewport before any overlay-induced paint"
        );
        drop(probe);
        app.shutdown();
    }
}
