//! 继承裁剪的重置使用实际菜单与应用结算；不改变来源文档。
use super::Application;
use crate::{desktop::image::Image, editor::Tool};
use windows::Win32::UI::WindowsAndMessaging::*;

pub(super) unsafe fn verify(app: &mut Application) {
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(Image {
                width: 8,
                height: 8,
                pixels: [10, 20, 30, 255].repeat(64).into(),
            })
            .unwrap();
        canvas
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.0,
                y: 1.0,
                width: 4.0,
                height: 4.0,
            })
            .unwrap();
    }
    app.sync_display_frames();
    let relay = &app.documents[2];
    let hwnd = relay.window.0;
    relay.state.canvas.borrow_mut().tool = Tool::Pan;
    assert!(relay.state.canvas.borrow().image.is_none());
    assert!(relay
        .state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .editor
        .as_ref()
        .unwrap()
        .crop_view()
        .is_some());
    let menu = CreatePopupMenu().unwrap();
    crate::desktop::crop_input::append(menu, &relay.state).unwrap();
    let flags = GetMenuState(menu, crate::desktop::crop_input::RESET as u32, MF_BYCOMMAND);
    DestroyMenu(menu).unwrap();
    assert_ne!(flags, u32::MAX);
    assert_eq!(flags & MF_GRAYED.0, 0);
    crate::desktop::context_menu::execute(hwnd, &relay.state, crate::desktop::crop_input::RESET)
        .unwrap();
    assert!(relay.state.pending_adoption.borrow().is_some());
    app.pending_windows(false).unwrap();
    let relay = &app.documents[2];
    assert!(relay.state.pending_adoption.borrow().is_none());
    assert!(relay.state.canvas.borrow().editor.crop_view().is_none());
    assert_eq!(relay.state.canvas.borrow().image.as_ref().unwrap().width, 8);
    assert!(app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .crop_view()
        .is_some());
    {
        let mut canvas = relay.state.canvas.borrow_mut();
        assert!(canvas.editor.undo());
        canvas.sync_raster_source();
        assert!(canvas.image.is_none());
    }
    app.sync_display_frames();
    assert!(app.documents[2]
        .state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .editor
        .as_ref()
        .unwrap()
        .crop_view()
        .is_some());
}
