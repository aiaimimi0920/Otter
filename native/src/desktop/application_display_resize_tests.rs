//! 显示来源标注经窗口选择后缩放，结算保留选择并维持一步撤销。
use super::Application;
use crate::editor::Tool;
use windows::Win32::{
    Foundation::LPARAM,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

pub(super) unsafe fn verify(app: &mut Application) {
    let relay = &app.documents[2];
    let hwnd = relay.window.0;
    let source = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    relay.state.canvas.borrow_mut().tool = Tool::Select;
    let viewport = relay.state.canvas.borrow().viewport;
    let dpi = GetDpiForWindow(hwnd) as f32 / 96.0;
    let x = ((viewport.origin.x + 2.0 * viewport.scale) * dpi).round() as i16;
    let y = ((viewport.origin.y + 2.0 * viewport.scale) * dpi).round() as i16;
    let point = LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize);
    SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point));
    SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point));
    assert_eq!(relay.state.canvas.borrow().selected_indices(), [0]);
    let width = relay.state.canvas.borrow().editor.elements()[0].width;
    app.pending_windows(false).unwrap();
    assert!(app.documents[2].state.pending_adoption.borrow().is_some());
    crate::desktop::resize_input::uniform(hwnd, &app.documents[2].state, 2.0);
    app.pending_windows(false).unwrap();
    let relay = &app.documents[2];
    assert!(relay.state.pending_adoption.borrow().is_none());
    {
        let mut canvas = relay.state.canvas.borrow_mut();
        assert_eq!(canvas.selected_indices(), [0]);
        assert!(canvas.editor.elements()[0].width > width);
        assert!(canvas.editor.undo());
        canvas.sync_raster_source();
        assert!(canvas.image.is_none());
    }
    assert!(source.matches(&app.documents[0].state.canvas.borrow().editor));
    app.sync_display_frames();
}
