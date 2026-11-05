//! 模糊与普通标注的外部显示组缩放；验证真实 worker 结算、选择和取消身份。
use super::Application;
use crate::{desktop::image::Image, editor::Tool, viewport::Point};
use windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::*};

pub(super) unsafe fn verify(app: &mut Application) {
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(Image {
                width: 16,
                height: 16,
                pixels: vec![255; 1024].into(),
            })
            .unwrap();
        let pixels = canvas.image.as_ref().unwrap().pixels.clone();
        canvas
            .editor
            .begin_blur(
                Point { x: 4.0, y: 8.0 },
                crate::blur_image::BlurImage::new(16, 16, pixels, 4).unwrap(),
            )
            .unwrap();
        assert!(canvas.editor.commit());
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 12.0, y: 8.0 })
            .unwrap();
        assert!(canvas.editor.commit());
    }
    app.sync_display_frames();
    for cancel in [false, true] {
        let relay = &app.documents[2];
        let hwnd = relay.window.0;
        let original = relay.state.canvas.borrow().editor.document_stamp();
        let identity = relay
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone();
        relay.state.canvas.borrow_mut().tool = Tool::Select;
        assert!(crate::desktop::display_edit::prepare_content(&relay.state));
        {
            let mut canvas = relay.state.canvas.borrow_mut();
            let revision = canvas.editor.revision();
            canvas.selected = Some((0, revision));
            canvas.multi_selection = Some((vec![0, 1], revision));
        }
        crate::desktop::resize_input::uniform(hwnd, &relay.state, 2.0);
        assert!(relay.state.resize_job.borrow().is_busy());
        let temporary = relay.state.canvas.borrow().editor.document_stamp();
        app.pending_windows(false).unwrap();
        assert!(temporary.matches(&app.documents[2].state.canvas.borrow().editor));
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        if cancel {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.documents[2].state.resize_job.borrow().is_busy() {
            SendMessageW(
                hwnd,
                WM_TIMER,
                Some(WPARAM(crate::desktop::resize_input::TIMER)),
                None,
            );
            app.pending_windows(false).unwrap();
            assert!(
                std::time::Instant::now() < deadline,
                "display blur resize timeout"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        app.pending_windows(false).unwrap();
        let relay = &app.documents[2];
        assert!(relay.state.pending_adoption.borrow().is_none());
        assert!(relay.state.error.borrow().is_none());
        {
            let source = app.documents[0].state.canvas.borrow();
            assert_eq!(
                source.editor.elements()[0].blur.as_ref().unwrap().sigma(),
                4
            );
            assert_eq!(source.editor.elements()[1].points[0].x, 12.0);
        }
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
            assert_eq!(canvas.selected_indices(), [0, 1]);
            assert_eq!(
                canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
                8
            );
            assert!(canvas.editor.elements()[1].points[0].x > 12.0);
            assert!(canvas.editor.undo());
            canvas.sync_raster_source();
            assert!(canvas.image.is_none());
        }
        app.sync_display_frames();
    }
}
