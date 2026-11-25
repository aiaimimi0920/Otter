//! Clip the same border HWND from complete image back to a ring; stopping input must not hide, destroy, or replace windows.
use super::*;
use std::cell::Cell;

struct Probe {
    disappeared: Cell<bool>,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const Probe);
    if message == WM_DESTROY
        || message == WM_NCDESTROY
        || (message == WM_SHOWWINDOW && wp.0 == 0)
        || (message == WM_WINDOWPOSCHANGING
            && lp.0 != 0
            && (*(lp.0 as *const WINDOWPOS)).flags.contains(SWP_HIDEWINDOW))
    {
        probe.disappeared.set(true);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

#[test]
fn idle_zoom_and_drag_keep_the_same_border_hwnd_visible_without_handoff() {
    unsafe {
        for renderer in [
            None,
            Some(desktop::render_commands::SOFTWARE),
            Some(desktop::render_commands::COMPATIBILITY),
        ] {
            for mode in [Mode::All, Mode::Border, Mode::None] {
                for dragging in [false, true] {
                    let mut app = visible_app();
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
                        canvas.normal_opacity = 0.5;
                        canvas.window_opacity = 0.5;
                    }
                    if let Some(command) = renderer {
                        desktop::render_commands::execute(doc.window.0, &doc.state, command)
                            .unwrap();
                    }
                    desktop::pin_window::fit(doc.window.0, &doc.state);
                    ui.mode.set(mode);
                    app.sync_workspace_chrome(true);
                    let doc = &app.documents[0];
                    let host = doc.window.0;
                    let choice = doc.state.canvas.borrow().render_choice();
                    if dragging {
                        SendMessageW(host, WM_ENTERSIZEMOVE, None, None);
                        SetWindowPos(
                            host,
                            None,
                            400,
                            300,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        )
                        .unwrap();
                    } else {
                        desktop::pin_window::wheel(
                            host,
                            &doc.state,
                            WPARAM(120 << 16),
                            LPARAM(280 | (220 << 16)),
                        );
                    }
                    let proxy = desktop::pin_window::preview::handle(&doc.state).unwrap();
                    atomic_tests::links_below_nodes(&ui);
                    if !dragging {
                        let mut before = RECT::default();
                        GetWindowRect(host, &mut before).unwrap();
                        let mut keys = [0; 256];
                        GetKeyboardState(&mut keys).unwrap();
                        let mut control = [0; 256];
                        control[VK_CONTROL.0 as usize] = 0x80;
                        SetKeyboardState(&control).unwrap();
                        SendMessageW(
                            proxy,
                            WM_MOUSEWHEEL,
                            Some(WPARAM((120 << 16) | 8)),
                            Some(LPARAM(280 | (220 << 16))),
                        );
                        SetKeyboardState(&keys).unwrap();
                        let mut after = RECT::default();
                        GetWindowRect(host, &mut after).unwrap();
                        assert!(
                            after.right - after.left > before.right - before.left,
                            "borrowed frame must forward actual wheel input"
                        );
                    }
                    let border = border_handle(&ui, 0);
                    let probe = Box::new(Probe {
                        disappeared: Cell::new(false),
                    });
                    if mode != Mode::None {
                        assert_eq!(proxy, border);
                    }
                    SetWindowSubclass(border, Some(observe), 557, &*probe as *const _ as usize)
                        .unwrap();
                    if dragging {
                        SendMessageW(host, WM_EXITSIZEMOVE, None, None);
                    } else {
                        desktop::pin_window::animation::settle(host, &doc.state);
                    }
                    // Retained frames are the normal idle path; continue protecting explicit-edit/cancel handoff here.
                    desktop::pin_window::animation::stop(host, &doc.state);
                    atomic_tests::links_below_nodes(&ui);
                    RemoveWindowSubclass(border, Some(observe), 557).unwrap();
                    assert!(
                        !probe.disappeared.get(),
                        "border HWND disappeared at idle handoff"
                    );
                    assert_eq!(IsWindow(Some(proxy)).as_bool(), mode != Mode::None);
                    assert!(desktop::pin_window::preview::handle(&doc.state).is_none());
                    if mode != Mode::None {
                        assert!(
                            WINDOW_EX_STYLE(GetWindowLongPtrW(border, GWL_EXSTYLE) as u32)
                                .contains(WS_EX_TRANSPARENT)
                        );
                        assert_eq!(
                            SendMessageW(border, WM_NCHITTEST, None, None).0,
                            HTTRANSPARENT as isize
                        );
                        let region = CreateRectRgn(0, 0, 0, 0);
                        assert_ne!(GetWindowRgn(border, region), NULLREGION);
                        assert!(PtInRegion(region, 0, 0).as_bool());
                        assert!(!PtInRegion(region, 10, 10).as_bool());
                        let _ = DeleteObject(region.into());
                    }
                    assert_eq!(IsWindowVisible(border).as_bool(), mode != Mode::None);
                    assert_eq!(doc.state.canvas.borrow().render_choice(), choice);
                    assert!(
                        doc.state.error.borrow().is_none(),
                        "{:?}",
                        doc.state.error.borrow()
                    );
                    assert!(
                        IsWindowVisible(border_handle(&ui, 1)).as_bool() == (mode != Mode::None)
                    );
                    app.shutdown();
                }
            }
        }
    }
}
