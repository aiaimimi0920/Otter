//! Observe every intermediate position of complete image/border frames and edit strips, not just final coordinates after host commit.
use super::*;
use std::time::Instant;
type PositionSample = (HWND, u32, (i32, i32));

#[derive(Default)]
struct Trace {
    host: HWND,
    message: Cell<u32>,
    expected: RefCell<Vec<(HWND, (i32, i32))>>,
    positions: RefCell<Vec<PositionSample>>,
    paints: Cell<u32>,
}
struct Watch {
    hwnd: HWND,
    _trace: Rc<Trace>,
}
impl Watch {
    unsafe fn new(hwnd: HWND, trace: &Rc<Trace>) -> Self {
        SetWindowSubclass(hwnd, Some(observe), 603, Rc::as_ptr(trace) as usize).unwrap();
        Self {
            hwnd,
            _trace: trace.clone(),
        }
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        unsafe {
            RemoveWindowSubclass(self.hwnd, Some(observe), 603).unwrap();
        }
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
    let trace = &*(data as *const Trace);
    let previous = trace.message.get();
    if hwnd == trace.host {
        trace.message.set(msg);
    } else if msg == WM_WINDOWPOSCHANGED && lp.0 != 0 {
        let pos = &*(lp.0 as *const WINDOWPOS);
        if !pos.flags.contains(SWP_NOMOVE) {
            trace
                .positions
                .borrow_mut()
                .push((hwnd, previous, origin(hwnd)));
        }
    } else if msg == WM_PAINT {
        trace.paints.set(trace.paints.get() + 1);
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    trace.message.set(previous);
    result
}

#[test]
fn shared_frame_and_toolbar_do_not_visit_old_positions_during_each_drag_step() {
    let _apartment = desktop::Apartment::new().unwrap();
    unsafe {
        for editing in [false, true] {
            let mut app = app();
            app.add_state(
                State::new(
                    Some(desktop::image::Image {
                        width: 160,
                        height: 120,
                        pixels: [20, 80, 160, 255].repeat(160 * 120).into(),
                    }),
                    true,
                )
                .unwrap(),
                false,
            )
            .unwrap();
            let doc = &app.documents[1];
            let host = doc.window.0;
            SetWindowPos(
                host,
                None,
                240,
                300,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
            app.sync_link_menus();
            app.sync_workspace_chrome(true);
            let doc = &app.documents[1];
            desktop::pin_window::fit(host, &doc.state);
            desktop::edit_mode::toolbar_mode(host, &doc.state, editing).unwrap();
            let panel = doc.panel.0;
            desktop::tool_panel::prepare(panel, &doc.state).unwrap();
            assert!(neuro_mini_ui::is_toolbar(panel));
            let _ = ShowWindow(panel, if editing { SW_SHOWNOACTIVATE } else { SW_HIDE });
            SendMessageW(host, WM_ENTERSIZEMOVE, None, None);
            let frame = desktop::pin_window::preview::handle(&doc.state).unwrap();
            let trace = Rc::new(Trace {
                host,
                ..Default::default()
            });
            let mut watches = vec![Watch::new(host, &trace), Watch::new(frame, &trace)];
            if editing {
                watches.push(Watch::new(panel, &trace));
            }
            let port_trace = Rc::new(Trace::default());
            for &(port, _, _) in app.chrome.ui.input_windows.borrow().iter() {
                UpdateWindow(port).ok().unwrap();
                watches.push(Watch::new(port, &port_trace));
            }
            let padding =
                desktop::pin_border_style::width(windows::Win32::UI::HiDpi::GetDpiForWindow(host))
                    as i32;
            let mut durations = Vec::new();
            let mut observed = 0;
            let submissions = app
                .chrome
                .ui
                .surface
                .borrow()
                .as_ref()
                .unwrap()
                .submissions
                .get();
            let paints = doc.state.canvas.borrow().paints;
            for step in 0..40 {
                let offset = if step < 20 { step } else { 38 - step };
                let (x, y) = (260 + offset * 3, 300 + offset * 2);
                trace.expected.borrow_mut().clear();
                trace
                    .expected
                    .borrow_mut()
                    .push((frame, (x - padding, y - padding)));
                if editing {
                    let proposed = RECT {
                        left: x,
                        top: y,
                        right: x + 160,
                        bottom: y + 120,
                    };
                    let attached =
                        desktop::tool_panel::moving_positions(host, &doc.state, proposed).unwrap();
                    assert!(attached.iter().any(|(h, _)| *h == panel));
                    trace
                        .expected
                        .borrow_mut()
                        .extend(attached.iter().map(|(h, r)| (*h, (r.left, r.top))));
                }
                trace.positions.borrow_mut().clear();
                let start = Instant::now();
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
                durations.push(start.elapsed().as_micros());
                let positions = trace.positions.borrow();
                assert!(positions.iter().any(|(h, _, _)| *h == frame));
                if editing {
                    assert!(positions.iter().any(|(h, _, _)| *h == panel));
                }
                observed += positions.len();
                for (hwnd, message, point) in positions.iter() {
                    let expected = trace
                        .expected
                        .borrow()
                        .iter()
                        .find(|(h, _)| h == hwnd)
                        .unwrap()
                        .1;
                    assert_eq!(*point, expected, "editing={editing} step={step} host_message={message:#x} sequence={positions:?}");
                }
            }
            assert_eq!(
                app.chrome
                    .ui
                    .surface
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .submissions
                    .get(),
                submissions,
                "moving a pin without changing any connection must not resubmit the shared bitmap"
            );
            assert_eq!(
                doc.state.canvas.borrow().paints,
                paints,
                "pure movement must reuse image pixels"
            );
            assert_eq!(
                port_trace.paints.get(),
                0,
                "unchanged port pixels must travel with their cached windows"
            );
            durations.sort();
            eprintln!(
                "drag-samples editing={editing} moves={observed} paints={} median_us={} max_us={}",
                trace.paints.get(),
                durations[20],
                durations[39]
            );
            drop(watches);
            SendMessageW(host, WM_EXITSIZEMOVE, None, None);
            assert!(doc.state.error.borrow().is_none());
            app.shutdown();
        }
    }
}

#[test]
fn connection_cache_redraws_changed_endpoints_and_clears_removed_lines() {
    unsafe {
        let mut app = visible_app();
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation { catalog, link });
        assert_eq!(app.workspace.links().len(), 1);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let ui = &app.chrome.ui;
        assert_eq!(ui.scene.borrow().as_ref().unwrap().edges.len(), 1);
        window::refresh_drawing(ui).unwrap();
        let count = || ui.surface.borrow().as_ref().unwrap().submissions.get();
        let start = count();
        window::refresh_drawing(ui).unwrap();
        assert_eq!(
            count(),
            start,
            "unchanged connected scene must reuse its submitted pixels"
        );

        let mut committed = RECT::default();
        GetWindowRect(ui.drawing.get(), &mut committed).unwrap();
        let shifted = Rect {
            left: committed.left + 1,
            top: committed.top,
            right: committed.right + 1,
            bottom: committed.bottom,
        };
        let failure = Error::new(E_FAIL, "injected curve failure");
        let error =
            draw::present_with(ui, shifted, draw::lines(ui), |_, _, _| Err(failure.clone()))
                .unwrap_err();
        assert_eq!(error.message(), failure.message());
        let mut after_failure = RECT::default();
        GetWindowRect(ui.drawing.get(), &mut after_failure).unwrap();
        assert_eq!(
            after_failure, committed,
            "a partial frame must not reach ULW"
        );
        assert!(
            ui.surface.borrow().is_none(),
            "partial DIB must not be cached"
        );
        window::refresh_drawing(ui).unwrap();
        let start = count();
        assert_eq!(start, 1, "unchanged scene must be submitted on retry");
        window::refresh_drawing(ui).unwrap();
        assert_eq!(count(), start, "success makes the completed frame reusable");

        let host = app.documents[0].window.0;
        SetWindowPos(
            host,
            None,
            260,
            200,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert!(
            count() > start,
            "moving a connected endpoint must update the curve immediately"
        );
        let moved = count();
        ui.scene.borrow_mut().take();
        window::refresh_drawing(ui).unwrap();
        assert!(
            count() > moved,
            "removing the last line must clear the old pixels"
        );
        let cleared = count();
        assert_eq!(
            ui.surface.borrow().as_ref().unwrap().size(),
            SIZE { cx: 1, cy: 1 },
            "removing the last curve must release the multi-pin DIB back to a 1x1 anchor"
        );
        window::refresh_drawing(ui).unwrap();
        assert_eq!(count(), cleared);
        app.shutdown();
    }
}

#[test]
fn unlinked_far_corner_pins_allocate_only_1x1_drawing_surface_until_connected() {
    unsafe {
        let mut app = visible_app();
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let right = vx + GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let bottom = vy + GetSystemMetrics(SM_CYVIRTUALSCREEN);
        SetWindowPos(
            app.documents[0].window.0,
            None,
            vx + 40,
            vy + 40,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        SetWindowPos(
            app.documents[1].window.0,
            None,
            (right - 220).max(vx + 240),
            (bottom - 180).max(vy + 200),
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        app.sync_workspace_chrome(true);
        let ui = app.chrome.ui.clone();
        assert_eq!(
            ui.surface.borrow().as_ref().unwrap().size(),
            SIZE { cx: 1, cy: 1 }
        );
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation { catalog, link });
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let linked = ui.surface.borrow().as_ref().unwrap().size();
        assert!(linked.cx > 1 && linked.cy > 1);
        app.add_window(false).unwrap();
        let third = app.documents[2].window.0;
        let _ = ShowWindow(third, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let nodes = ui.nodes.borrow().clone();
        let strip = ui
            .input_windows
            .borrow()
            .iter()
            .find(|(_, n, _)| *n == 2)
            .unwrap()
            .0;
        {
            let mut rows = ui.nodes.borrow_mut();
            for row in rows.iter_mut().filter(|n| n.endpoint < 2) {
                let w = row.rect.right - row.rect.left;
                row.rect.left = right + 1000 + row.endpoint as i32 * 300;
                row.rect.right = row.rect.left + w;
            }
        }
        window::refresh_drawing(&ui).unwrap();
        assert_eq!(
            ui.surface.borrow().as_ref().unwrap().size(),
            SIZE { cx: 1, cy: 1 }
        );
        assert!(
            IsWindowVisible(strip).as_bool(),
            "offscreen links must not hide an unrelated port strip"
        );
        *ui.nodes.borrow_mut() = nodes;
        window::refresh_drawing(&ui).unwrap();
        assert!(ui.surface.borrow().as_ref().unwrap().size().cx > 1);
        ui.nodes.borrow_mut().clear();
        window::refresh_drawing(&ui).unwrap();
        assert!(ui.surface.borrow().is_none());
        app.shutdown();
    }
}
