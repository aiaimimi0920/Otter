//! Actual cached-movement entry: image/border and this host's two ports move together while other nodes stay put.
use super::*;
#[path = "workspace_chrome_drag_tests.rs"]
mod drag_tests;

unsafe fn origin(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect).unwrap();
    (rect.left, rect.top)
}

#[test]
fn cached_frame_moves_both_ports_in_the_same_batch_without_waiting_for_chrome_refresh() {
    unsafe {
        let mut app = visible_app();
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
        let frame = desktop::pin_window::preview::handle(&doc.state).unwrap();
        let left = port(&app.chrome.ui, 0, Side::Input).0;
        let right = port(&app.chrome.ui, 0, Side::Output).0;
        let other = port(&app.chrome.ui, 1, Side::Input).0;
        let initial = [origin(frame), origin(left), origin(right), origin(other)];
        let mut rect = RECT::default();
        GetWindowRect(host, &mut rect).unwrap();
        for (dx, dy) in [(37, 11), (-90, 44), (140, -65)] {
            // Test preview-position entry directly before node.rect reaches zoom_frame for updating.
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
            for (index, hwnd) in [frame, left, right].into_iter().enumerate() {
                assert_eq!(origin(hwnd), (initial[index].0 + dx, initial[index].1 + dy));
            }
            assert_eq!(origin(other), initial[3]);
        }
        SendMessageW(host, WM_CANCELMODE, None, None);
        app.shutdown();
    }
}
