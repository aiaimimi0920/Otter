//! 来源标注的全部栅格化：真实后台渲染、窗口定时器、暂存结算与取消。
use super::Application;
use crate::{editor::Tool, viewport::Point};
use windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::*};

pub(super) unsafe fn verify(app: &mut Application) {
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 2.0, y: 2.0 })
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
        let menu = CreatePopupMenu().unwrap();
        crate::desktop::rasterize_input::append_menu(menu, &relay.state).unwrap();
        let flags = GetMenuState(
            menu,
            crate::desktop::rasterize_input::ALL as u32,
            MF_BYCOMMAND,
        );
        DestroyMenu(menu).unwrap();
        assert_ne!(flags, u32::MAX);
        assert_eq!(flags & MF_GRAYED.0, 0);
        crate::desktop::context_menu::execute(
            hwnd,
            &relay.state,
            crate::desktop::rasterize_input::ALL,
        )
        .unwrap();
        assert!(relay.state.rasterize_job.borrow().is_some());
        app.pending_windows(false).unwrap();
        assert!(app.documents[2].state.pending_adoption.borrow().is_some());
        if cancel {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.documents[2].state.rasterize_job.borrow().is_some() {
            SendMessageW(
                hwnd,
                WM_TIMER,
                Some(WPARAM(crate::desktop::rasterize_input::TIMER)),
                None,
            );
            app.pending_windows(false).unwrap();
            assert!(
                std::time::Instant::now() < deadline,
                "display rasterize timeout"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        app.pending_windows(false).unwrap();
        let relay = &app.documents[2];
        assert!(relay.state.pending_adoption.borrow().is_none());
        assert!(relay.state.error.borrow().is_none());
        assert_eq!(
            app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .elements()
                .len(),
            1
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
            assert!(canvas.editor.elements().is_empty());
            assert!(canvas
                .editor
                .layers()
                .unwrap()
                .annotations
                .as_ref()
                .unwrap()
                .chunks_exact(4)
                .any(|pixel| pixel[3] != 0));
            assert!(canvas.editor.undo());
            canvas.sync_raster_source();
            assert!(canvas.image.is_none());
        }
        app.sync_display_frames();
    }
}
