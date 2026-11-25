//! Continuous/intermittent wheels on translucent pins share one visible frame; idle must not restore a second pixel copy.
use super::*;
use desktop::pin_window::{animation, preview, stationary};

unsafe fn masked(hwnd: HWND) -> bool {
    let region = CreateRectRgn(0, 0, 0, 0);
    let result = GetWindowRgn(hwnd, region) == NULLREGION;
    let _ = DeleteObject(region.into());
    result
}

#[test]
fn translucent_idle_reuses_one_frame_and_refreshes_opacity_without_blocking_save() {
    unsafe {
        for backend in [
            None,
            Some(desktop::render_commands::SOFTWARE),
            Some(desktop::render_commands::COMPATIBILITY),
        ] {
            let mut app = visible_app();
            let doc = &app.documents[0];
            let host = doc.window.0;
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
            if let Some(command) = backend {
                desktop::render_commands::execute(host, &doc.state, command).unwrap();
            }
            desktop::pin_window::fit(host, &doc.state);
            let _ = UpdateWindow(host);
            app.sync_workspace_chrome(true);
            let doc = &app.documents[0];
            let choice = doc.state.canvas.borrow().render_choice();
            let paints = doc.state.canvas.borrow().paints;
            let border = border_handle(&app.chrome.ui, 0);
            for delta in [120i16, -120, 120, -120] {
                desktop::pin_window::wheel(
                    host,
                    &doc.state,
                    WPARAM((delta as u16 as usize) << 16),
                    LPARAM(280 | (220 << 16)),
                );
                assert_eq!(preview::handle(&doc.state), Some(border));
                assert!(masked(host));
                animation::settle(host, &doc.state);
                assert!(stationary::active(&doc.state));
                assert!(
                    !animation::busy(&doc.state),
                    "a static retained frame must not block autosave"
                );
                assert_eq!(preview::handle(&doc.state), Some(border));
                assert!(
                    masked(host),
                    "idle exposed a second copy of translucent pixels"
                );
                assert_eq!(
                    doc.state.canvas.borrow().paints,
                    paints,
                    "idle must not redraw and expose another backend"
                );
            }
            doc.state.canvas.borrow_mut().normal_opacity = 0.3;
            doc.state.canvas.borrow_mut().window_opacity = 0.3;
            let _ = InvalidateRect(Some(host), None, false);
            let _ = UpdateWindow(host);
            assert!(preview::fresh(&doc.state));
            assert!(masked(host));
            assert_eq!(preview::handle(&doc.state), Some(border));
            assert_eq!(doc.state.canvas.borrow().render_choice(), choice);
            assert!(
                doc.state.error.borrow().is_none(),
                "{:?}",
                doc.state.error.borrow()
            );
            let _ = ShowWindow(host, SW_HIDE);
            assert!(!preview::active(&doc.state));
            let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
            assert!(
                !masked(host),
                "show must not leave a masked host with no retained frame"
            );
            app.shutdown();
        }
    }
}

#[test]
fn hiding_chrome_restores_parked_content_before_hiding_decorations() {
    unsafe {
        let mut app = visible_app();
        let doc = &app.documents[0];
        let host = doc.window.0;
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
        desktop::pin_window::fit(host, &doc.state);
        desktop::pin_window::wheel(
            host,
            &doc.state,
            WPARAM(120 << 16),
            LPARAM(280 | (220 << 16)),
        );
        animation::settle(host, &doc.state);
        assert!(stationary::active(&doc.state));
        window::hide(&app.chrome.ui);
        assert!(!preview::active(&doc.state));
        assert!(!masked(host));
        assert!(IsWindowVisible(host).as_bool());
        assert!(
            doc.state.error.borrow().is_none(),
            "{:?}",
            doc.state.error.borrow()
        );
        app.shutdown();
    }
}
