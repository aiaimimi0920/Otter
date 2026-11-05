//! 带模糊来源的真实裁剪拖拽、后台重算、取消与一步撤销。
use super::Application;
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

pub(super) unsafe fn verify(app: &mut Application) {
    // 裁剪改变栅格标注的可见框时才需要重新合成模糊；仅底图裁剪走同步路径。
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        let original = canvas.editor.layers().unwrap().clone();
        let mut layers = original.clone();
        layers.annotations = Some([0, 0, 255, 255].repeat(256).into());
        let revision = canvas.editor.revision();
        assert!(canvas
            .editor
            .replace_layers(revision, &original, layers)
            .unwrap());
    }
    app.sync_display_frames();
    for cancel in [false, true] {
        let relay = &app.documents[2];
        let hwnd = relay.window.0;
        relay.state.canvas.borrow_mut().crop_mode = false;
        let original = relay.state.canvas.borrow().editor.document_stamp();
        let identity = relay
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone();
        let source_blur = app.documents[0].state.canvas.borrow().editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .clone();
        crate::desktop::context_menu::execute(
            hwnd,
            &relay.state,
            crate::desktop::crop_input::START,
        )
        .unwrap();
        let viewport = relay.state.canvas.borrow().viewport;
        let dpi = GetDpiForWindow(hwnd) as f32 / 96.0;
        let point = |x: f32, y: f32| {
            let x = ((viewport.origin.x + x * viewport.scale) * dpi).round() as i16;
            let y = ((viewport.origin.y + y * viewport.scale) * dpi).round() as i16;
            LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize)
        };
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point(2.0, 2.0)));
        assert!(
            app.documents[2].state.canvas.borrow().crop_drag.is_some(),
            "crop drag did not start"
        );
        app.pending_windows(false).unwrap();
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        SendMessageW(hwnd, WM_MOUSEMOVE, None, Some(point(12.0, 12.0)));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point(12.0, 12.0)));
        assert!(
            app.documents[2].state.crop_job.borrow().is_some(),
            "crop absent: view={:?}, mode={}, pending={}",
            app.documents[2].state.canvas.borrow().editor.crop_view(),
            app.documents[2].state.canvas.borrow().crop_mode,
            app.documents[2]
                .state
                .canvas
                .borrow()
                .crop_pending
                .is_some()
        );
        app.pending_windows(false).unwrap();
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        assert!(app.documents[2]
            .state
            .canvas
            .borrow()
            .editor
            .crop_view()
            .is_none());
        if cancel {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.documents[2].state.crop_job.borrow().is_some() {
            SendMessageW(
                hwnd,
                WM_TIMER,
                Some(WPARAM(crate::desktop::crop_apply::TIMER)),
                None,
            );
            app.pending_windows(false).unwrap();
            assert!(
                std::time::Instant::now() < deadline,
                "display blur crop timeout"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        app.pending_windows(false).unwrap();
        let relay = &app.documents[2];
        assert!(relay.state.pending_adoption.borrow().is_none());
        assert!(relay.state.error.borrow().is_none());
        assert!(app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .crop_view()
            .is_none());
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
            assert!(canvas.editor.crop_view().is_some());
            assert!(!canvas.editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(source_blur.pixels()));
            assert!(canvas.editor.undo());
            canvas.sync_raster_source();
            assert!(canvas.image.is_none());
        }
        app.sync_display_frames();
    }
}
