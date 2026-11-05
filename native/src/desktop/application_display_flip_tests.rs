//! 外部显示的菜单翻转：经过后台任务、窗口定时器与应用结算，原来源不可被修改。
use super::Application;
use crate::{desktop::image::Image, editor::Tool};
use windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::*};

pub(super) unsafe fn verify(app: &mut Application) {
    let pixels = vec![1, 2, 3, 255, 9, 8, 7, 255];
    app.documents[0]
        .state
        .canvas
        .borrow_mut()
        .replace_image(Image {
            width: 2,
            height: 1,
            pixels: pixels.clone().into(),
        })
        .unwrap();
    app.sync_display_frames();
    for cancel in [false, true] {
        let relay = &app.documents[2];
        let hwnd = relay.window.0;
        relay.state.canvas.borrow_mut().tool = Tool::Pan;
        let original = relay.state.canvas.borrow().editor.document_stamp();
        let identity = relay
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone();
        crate::desktop::context_menu::execute(
            hwnd,
            &relay.state,
            crate::desktop::flip_input::HORIZONTAL,
        )
        .unwrap();
        assert!(relay.state.flip_job.borrow().is_busy());
        assert!(relay.state.pending_adoption.borrow().is_some());
        app.pending_windows(false).unwrap();
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        if cancel {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.documents[2].state.flip_job.borrow().is_busy() {
            SendMessageW(
                hwnd,
                WM_TIMER,
                Some(WPARAM(crate::desktop::flip_input::TIMER)),
                None,
            );
            app.pending_windows(false).unwrap();
            assert!(std::time::Instant::now() < deadline, "display flip timeout");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        app.pending_windows(false).unwrap();
        let relay = &app.documents[2];
        assert!(relay.state.pending_adoption.borrow().is_none());
        assert!(relay.state.error.borrow().is_none());
        assert_eq!(
            &*app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels,
            &pixels
        );
        if cancel {
            assert!(original.matches(&relay.state.canvas.borrow().editor));
            assert!(std::sync::Arc::ptr_eq(
                &identity,
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
            assert_eq!(
                &*canvas.image.as_ref().unwrap().pixels,
                &[9, 8, 7, 255, 1, 2, 3, 255]
            );
            assert!(canvas.editor.undo());
            canvas.sync_raster_source();
            assert!(canvas.image.is_none());
        }
        app.sync_display_frames();
    }
}
