//! Production window gestures/timers/main loop verify staged takeover and asynchronous erasure, without accessing user windows or the clipboard.
use super::Application;
use crate::editor::Tool;
use std::time::{Duration, Instant};
use windows::Win32::{
    Foundation::{LPARAM, RECT, WPARAM},
    UI::WindowsAndMessaging::*,
};

pub(super) unsafe fn verify(app: &mut Application) {
    for cancel in [false, true] {
        let relay = &app.documents[2];
        let hwnd = relay.window.0;
        relay.state.canvas.borrow_mut().tool = Tool::Eraser;
        let original = relay.state.canvas.borrow().editor.document_stamp();
        let display = relay
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone();
        let source = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        // Centered display may leave transparent space at the upper-left corner; erasure must start inside actual content.
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client).unwrap();
        let point = LPARAM((client.right / 2) as isize | (((client.bottom / 2) as isize) << 16));
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point));
        assert!(relay.state.erase_job.borrow().is_busy());
        assert!(relay.state.pending_adoption.borrow().is_some());
        let temporary = relay.state.canvas.borrow().editor.document_stamp();
        app.pending_windows(false).unwrap();
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        assert!(temporary.matches(&app.documents[2].state.canvas.borrow().editor));
        if cancel {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.documents[2].state.erase_job.borrow().is_busy() {
            SendMessageW(
                hwnd,
                WM_TIMER,
                Some(WPARAM(crate::desktop::erase_input::TIMER)),
                None,
            );
            app.pending_windows(false).unwrap();
            assert!(Instant::now() < deadline, "display adoption erase timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        app.pending_windows(false).unwrap();
        let relay = &app.documents[2];
        assert!(relay.state.pending_adoption.borrow().is_none());
        assert!(relay.state.canvas.borrow().erase_display.is_none());
        assert!(relay.state.error.borrow().is_none());
        assert!(source.matches(&app.documents[0].state.canvas.borrow().editor));
        if cancel {
            assert!(original.matches(&relay.state.canvas.borrow().editor));
            assert!(std::sync::Arc::ptr_eq(
                &display,
                &relay
                    .state
                    .resolved_display
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .identity
            ));
        } else {
            let mut canvas = relay.state.canvas.borrow_mut();
            assert_eq!(canvas.image.as_ref().unwrap().pixels[3], 0);
            assert!(canvas.editor.undo());
            canvas.sync_raster_source();
            assert!(canvas.image.is_none());
        }
        app.sync_display_frames();
        assert!(app.documents[2].state.resolved_display.borrow().is_some());
    }
}
