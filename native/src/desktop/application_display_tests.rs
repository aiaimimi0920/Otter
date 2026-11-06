//! 用生产应用 owner 验证断链、来源编辑与撤销后的显示快照生命周期。
use super::Application;
use crate::{desktop::image::Image, editor::Tool, viewport::Point};

pub(super) fn verify_display_edit(app: &mut Application) {
    let relay = &app.documents[2];
    let source_stamp = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    relay.state.canvas.borrow_mut().tool = Tool::Pan;
    assert!(crate::desktop::display_edit::prepare(&relay.state));
    assert!(relay.state.resolved_display.borrow().is_some());
    relay.state.group_locked.set(true);
    relay.state.canvas.borrow_mut().tool = Tool::Brush;
    assert!(!crate::desktop::display_edit::prepare(&relay.state));
    assert!(relay.state.canvas.borrow().image.is_none());
    relay.state.group_locked.set(false);
    let viewport = relay.state.canvas.borrow().viewport;
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        let mut rect = windows::Win32::Foundation::RECT::default();
        GetClientRect(relay.window.0, &mut rect).unwrap();
        // 从实际居中内容内部开始编辑，而不是点击透明留白的左上角。
        let point = windows::Win32::Foundation::LPARAM(
            (rect.right / 2) as isize | (((rect.bottom / 2) as isize) << 16),
        );
        SendMessageW(relay.window.0, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(relay.window.0, WM_LBUTTONUP, None, Some(point));
    }
    assert!(relay.state.resolved_display.borrow().is_none());
    assert!(relay.state.pending_adoption.borrow().is_some());
    unsafe {
        app.pending_windows(false).unwrap();
    }
    let relay = &app.documents[2];
    assert!(relay.state.pending_adoption.borrow().is_none());
    {
        let mut canvas = relay.state.canvas.borrow_mut();
        assert_eq!(canvas.viewport, viewport);
        assert_eq!(&*canvas.image.as_ref().unwrap().pixels, &[10, 20, 30, 255]);
        assert_eq!(canvas.editor.elements().len(), 1);
        assert!(canvas.editor.undo()); // 本次笔画与接管为同一步
        canvas.sync_raster_source();
        assert!(canvas.image.is_none());
        assert!(canvas.editor.layers().is_none());
    }
    assert!(source_stamp.matches(&app.documents[0].state.canvas.borrow().editor));
    app.sync_display_frames();
    assert!(app.documents[2].state.resolved_display.borrow().is_some());
}

pub(super) fn verify_display_lifetime(app: &mut Application) {
    let relay_id = app.documents[2].state.unit_id.clone();
    let link = app
        .workspace
        .links()
        .iter()
        .find(|link| link.to_unit == relay_id)
        .unwrap()
        .clone();
    assert!(app.workspace.remove_link(&link.id));
    app.sync_display_frames();
    assert!(app.documents[2].state.resolved_display.borrow().is_none());
    app.workspace.connect(link).unwrap();
    app.sync_display_frames();
    assert!(app.documents[2].state.resolved_display.borrow().is_some());

    // 把来源切换为本地已编辑钉图，确保旧 Art 正式输出不再成为显示来源。
    let source = &app.documents[0].state;
    source.selected_art.borrow_mut().take();
    {
        let mut canvas = source.canvas.borrow_mut();
        canvas
            .replace_image(Image {
                width: 1,
                height: 1,
                pixels: vec![10, 20, 30, 255].into(),
            })
            .unwrap();
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 0.0, y: 0.0 })
            .unwrap();
        canvas.editor.extend(Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(canvas.editor.commit());
    }
    app.sync_display_frames();
    let old = app.documents[2]
        .state
        .resolved_display
        .borrow_mut()
        .take()
        .unwrap();
    assert_eq!(&*old.image.pixels, &[10, 20, 30, 255]);
    assert_eq!(old.editor.as_ref().unwrap().elements().len(), 1);
    assert!(app.documents[2].state.canvas.borrow().image.is_none());

    assert!(source.canvas.borrow_mut().editor.undo());
    app.sync_display_frames();
    let current = app.documents[2].state.resolved_display.borrow();
    let current = current.as_ref().unwrap();
    assert!(current.editor.as_ref().unwrap().elements().is_empty());
    assert!(current.image.pixels.same_storage(&old.image.pixels));
    // 新快照不能反向修改仍被消费者持有的旧编辑快照。
    assert_eq!(old.editor.as_ref().unwrap().elements().len(), 1);
    let relay = &app.documents[2];
    let mut canvas = relay.state.canvas.borrow_mut();
    let original = canvas.editor.document_stamp();
    let paints = canvas.paints;
    for frame in [&old, current] {
        canvas
            .paint_display(relay.window.0, 64, 64, 96, Some(frame))
            .unwrap();
        assert!(canvas.image.is_none());
        assert!(original.matches(&canvas.editor));
    }
    canvas.paint(relay.window.0, 64, 64, 96).unwrap();
    assert_eq!(canvas.paints, paints + 3);
    drop(canvas);
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_PAINT};
        SendMessageW(relay.window.0, WM_PAINT, None, None);
    }
    assert!(relay.state.error.borrow().is_none());
    let mut canvas = relay.state.canvas.borrow_mut();
    assert_eq!(canvas.paints, paints + 4);
    assert!(canvas.image.is_none() && original.matches(&canvas.editor));
    canvas.pinned = true;
    canvas.tool = Tool::Pan;
    drop(canvas);
    unsafe {
        use windows::Win32::{Foundation::RECT, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
        crate::desktop::pin_window::fit(relay.window.0, &relay.state);
        let mut client = RECT::default();
        GetClientRect(relay.window.0, &mut client).unwrap();
        let canvas = relay.state.canvas.borrow();
        assert!(canvas.viewport.scale > 1.0);
        assert!(canvas.image.is_none());
        drop(canvas);
        crate::desktop::hit_region::update(relay.window.0, &relay.state).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert!(!region.is_invalid());
        let kind = GetWindowRgn(relay.window.0, region);
        let contains = PtInRegion(region, client.right / 2, client.bottom / 2).as_bool();
        let outside = PtInRegion(region, client.right + 1, client.bottom + 1).as_bool();
        let _ = DeleteObject(region.into());
        assert_ne!(kind, RGN_ERROR);
        assert!(contains && !outside);
        let mut window = RECT::default();
        GetWindowRect(relay.window.0, &mut window).unwrap();
        let border = (
            window.right - window.left - client.right,
            window.bottom - window.top - client.bottom,
        );
        for (width, height) in [(200, 100), (100, 200)] {
            SetWindowPos(
                relay.window.0,
                None,
                0,
                0,
                width + border.0,
                height + border.1,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            GetClientRect(relay.window.0, &mut client).unwrap();
            // 旧夹具是 captioned HWND，系统最小宽度可能扩大客户区；按真实尺寸核居中。
            let span = client.right.min(client.bottom);
            let left = ((client.right - span) as f32 * 0.5).round() as i32;
            let top = ((client.bottom - span) as f32 * 0.5).round() as i32;
            crate::desktop::hit_region::update(relay.window.0, &relay.state).unwrap();
            let region = CreateRectRgn(0, 0, 0, 0);
            assert!(!region.is_invalid());
            let kind = GetWindowRgn(relay.window.0, region);
            let mut bounds = RECT::default();
            let bounds_kind = GetRgnBox(region, &mut bounds);
            let centered = PtInRegion(region, client.right / 2, client.bottom / 2).as_bool();
            let _ = DeleteObject(region.into());
            assert_ne!(kind, RGN_ERROR);
            assert_ne!(bounds_kind, RGN_ERROR);
            assert!(centered);
            assert_eq!(
                (bounds.left, bounds.top, bounds.right, bounds.bottom),
                (left, top, left + span, top + span),
                "relay hit region must follow centered content in actual client {}x{} (requested {width}x{height})",
                client.right, client.bottom
            );
            assert!(relay.state.canvas.borrow().image.is_none());
        }
        SetWindowPos(
            relay.window.0,
            None,
            0,
            0,
            window.right - window.left,
            window.bottom - window.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
}
