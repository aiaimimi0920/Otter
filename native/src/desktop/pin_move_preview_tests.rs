//! The same cached image/border window retains its geometry during system moves; completion/cancellation/destruction leave no proxy.
use super::*;

#[test]
fn cached_move_preserves_backend_and_releases_on_exit_cancel_and_destroy() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for renderer in [None, Some(crate::desktop::render_commands::SOFTWARE)] {
            for event in [WM_EXITSIZEMOVE, WM_CANCELMODE, WM_CLOSE] {
                let fixture = Fixture::with_renderer(renderer);
                let choice = fixture.state.canvas.borrow().render_choice();
                let stamp = fixture.state.canvas.borrow().editor.document_stamp();
                let paints = fixture.state.canvas.borrow().paints;
                SendMessageW(fixture.window.0, WM_ENTERSIZEMOVE, None, None);
                let proxy = preview::handle(&fixture.state).unwrap();
                let padding =
                    crate::desktop::pin_border_style::width(GetDpiForWindow(fixture.window.0))
                        as i32;
                for index in 0..100 {
                    SetWindowPos(
                        fixture.window.0,
                        None,
                        100 + index * 3,
                        100 + index % 4 * 60,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                    .unwrap();
                    let host = fixture.rect();
                    let mut bounds = RECT::default();
                    GetWindowRect(proxy, &mut bounds).unwrap();
                    assert_eq!(bounds.left, host.x - padding);
                    assert_eq!(bounds.top, host.y - padding);
                    assert_eq!(bounds.right, host.x + host.width as i32 + padding);
                    assert_eq!(bounds.bottom, host.y + host.height as i32 + padding);
                    assert_eq!(fixture.state.canvas.borrow().paints, paints);
                }
                SetWindowPos(
                    fixture.window.0,
                    None,
                    0,
                    0,
                    220,
                    170,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
                let mut resized = RECT::default();
                GetWindowRect(proxy, &mut resized).unwrap();
                assert_eq!(resized.right - resized.left, 220 + 2 * padding);
                assert_eq!(resized.bottom - resized.top, 170 + 2 * padding);
                SendMessageW(fixture.window.0, event, None, None);
                assert!(!IsWindow(Some(proxy)).as_bool());
                assert!(!preview::active(&fixture.state));
                assert!(!animation::busy(&fixture.state));
                assert_eq!(fixture.state.canvas.borrow().render_choice(), choice);
                assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
                assert!(fixture.state.error.borrow().is_none());
            }
        }
    }
}
