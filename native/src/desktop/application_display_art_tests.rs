//! Art 本地标注只影响自身显示快照，正式图片及下游中继仍保持原始结果。
use super::Application;
use crate::{editor::Tool, viewport::Point};

pub(super) fn verify(app: &mut Application) {
    verify_connected_formal_overlay(app);
    let raw = app.documents[0]
        .state
        .execution_results
        .borrow()
        .image_source("output")
        .unwrap()
        .image
        .clone();
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(crate::desktop::image::Image {
                width: 1,
                height: 1,
                pixels: vec![255, 0, 0, 255].into(),
            })
            .unwrap();
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 0.0, y: 0.0 })
            .unwrap();
        assert!(canvas.editor.commit());
    }
    app.sync_display_frames();
    {
        let display = app.documents[0].state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert_eq!(frame.editor.as_ref().unwrap().elements().len(), 1);
        assert!(frame.image.pixels.same_storage(&raw.pixels));
        assert!(app.documents[2]
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .editor
            .is_none());
        let state = &app.documents[0].state;
        let (_, snapshot) =
            crate::desktop::display_snapshot::capture(&state.canvas.borrow(), Some(frame)).unwrap();
        assert_eq!(snapshot.elements().len(), 1);
        assert!(
            snapshot.layers().is_none(),
            "old raster must not override generated display during export"
        );
    }
    let results = app.documents[0].state.execution_results.borrow();
    assert!(results
        .image_source("output")
        .unwrap()
        .image
        .pixels
        .same_storage(&raw.pixels));
    drop(results);
    assert!(app.documents[0].state.canvas.borrow_mut().editor.undo());
    app.sync_display_frames();
    assert!(app.documents[0]
        .state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .editor
        .as_ref()
        .unwrap()
        .elements()
        .is_empty());
    verify_pointer_edit(app, &raw);
    verify_crop_reset(app, &raw);
    verify_empty_overlay_rebind(app, &raw);
}

fn verify_connected_formal_overlay(app: &mut Application) {
    // 第二个 Art 只有标量输出，显示经透明钉图连接的第一个 Art 正式图片。
    let state = &app.documents[1].state;
    state.canvas.borrow_mut().tool = Tool::Brush;
    let original = state.canvas.borrow().editor.document_stamp();
    let source_stamp = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    let raw = app.documents[0]
        .state
        .execution_results
        .borrow()
        .image_source("output")
        .unwrap()
        .image
        .clone();
    let output_stamp = state.execution_results.borrow().output_stamp();
    {
        let display = state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert!(!frame.is_from(&state.unit_id));
        assert!(frame.has_overlay_owner(&state.unit_id));
        assert!(frame.image.pixels.same_storage(&raw.pixels));
    }
    assert!(crate::desktop::display_edit::prepare(state));
    assert!(crate::desktop::display_edit::settle(state));
    assert!(original.matches(&state.canvas.borrow().editor));
    assert!(crate::desktop::display_edit::prepare(state));
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas.editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(canvas.editor.commit());
    }
    assert!(crate::desktop::display_edit::settle(state));
    app.sync_display_frames();
    let state = &app.documents[1].state;
    {
        let display = state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert!(frame.has_overlay_owner(&state.unit_id));
        assert!(frame.image.pixels.same_storage(&raw.pixels));
        assert_eq!(frame.editor.as_ref().unwrap().elements().len(), 1);
        assert!(frame.editor.as_ref().unwrap().layers().is_none());
    }
    assert!(output_stamp.matches(&state.execution_results.borrow()));
    assert!(source_stamp.matches(&app.documents[0].state.canvas.borrow().editor));
    assert!(state.canvas.borrow_mut().editor.undo());
    assert!(state.canvas.borrow().editor.layers().is_none());
    app.sync_display_frames();
    assert!(app.documents[1]
        .state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .editor
        .as_ref()
        .unwrap()
        .elements()
        .is_empty());
}

fn verify_empty_overlay_rebind(app: &mut Application, raw: &crate::desktop::image::Image) {
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(crate::desktop::image::Image {
                width: 2,
                height: 2,
                pixels: vec![255; 16].into(),
            })
            .unwrap();
        canvas.tool = Tool::Brush;
    }
    app.sync_display_frames();
    let state = &app.documents[0].state;
    assert!(crate::desktop::display_edit::prepare(state));
    {
        let mut canvas = state.canvas.borrow_mut();
        assert_eq!(
            canvas.editor.layers().unwrap().size,
            (raw.width, raw.height)
        );
        assert!(canvas
            .editor
            .layers()
            .unwrap()
            .base
            .same_storage(&raw.pixels));
        canvas.editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(canvas.editor.commit());
    }
    unsafe {
        app.pending_windows(false).unwrap();
    }
    let state = &app.documents[0].state;
    assert!(state.pending_adoption.borrow().is_none());
    assert_eq!(state.canvas.borrow().editor.elements().len(), 1);
    assert!(state
        .execution_results
        .borrow()
        .image_source("output")
        .unwrap()
        .image
        .pixels
        .same_storage(&raw.pixels));
    let mut canvas = state.canvas.borrow_mut();
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.layers().unwrap().size, (2, 2));
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    assert!(canvas.editor.commit());
    drop(canvas);
    app.sync_display_frames();
    let state = &app.documents[0].state;
    let stamp = state.canvas.borrow().editor.document_stamp();
    assert_eq!(
        state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .elements()[0]
            .points[0],
        Point { x: 0.5, y: 0.5 }
    );
    assert!(crate::desktop::display_edit::prepare(state));
    assert_eq!(
        state.canvas.borrow().editor.elements()[0].points[0],
        Point { x: 0.5, y: 0.5 }
    );
    assert!(crate::desktop::display_edit::settle(state));
    assert!(stamp.matches(&state.canvas.borrow().editor));
    {
        let mut canvas = state.canvas.borrow_mut();
        assert!(canvas.editor.undo());
        canvas
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            })
            .unwrap();
    }
    app.sync_display_frames();
    let state = &app.documents[0].state;
    let stamp = state.canvas.borrow().editor.document_stamp();
    assert!(crate::desktop::display_edit::prepare(state));
    assert_eq!(
        state
            .canvas
            .borrow()
            .editor
            .crop_view()
            .unwrap()
            .frame
            .width,
        0.5
    );
    assert!(crate::desktop::display_edit::settle(state));
    assert!(stamp.matches(&state.canvas.borrow().editor));
}

fn verify_crop_reset(app: &mut Application, raw: &crate::desktop::image::Image) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 0.5,
            })
            .unwrap();
        canvas.tool = Tool::Pan;
    }
    app.sync_display_frames();
    let doc = &app.documents[0];
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        crate::desktop::crop_input::append(menu, &doc.state).unwrap();
        let flags = GetMenuState(menu, crate::desktop::crop_input::RESET as u32, MF_BYCOMMAND);
        DestroyMenu(menu).unwrap();
        assert_ne!(flags, u32::MAX);
        assert_eq!(flags & MF_GRAYED.0, 0);
        crate::desktop::context_menu::execute(
            doc.window.0,
            &doc.state,
            crate::desktop::crop_input::RESET,
        )
        .unwrap();
        app.pending_windows(false).unwrap();
    }
    let state = &app.documents[0].state;
    assert!(state.pending_adoption.borrow().is_none());
    assert!(state.canvas.borrow().editor.crop_view().is_none());
    assert!(state
        .execution_results
        .borrow()
        .image_source("output")
        .unwrap()
        .image
        .pixels
        .same_storage(&raw.pixels));
    assert!(state.canvas.borrow_mut().editor.undo());
    assert!(state.canvas.borrow().editor.crop_view().is_some());
    app.sync_display_frames();
    let display = app.documents[0].state.resolved_display.borrow();
    let frame = display.as_ref().unwrap();
    assert!(frame.editor.as_ref().unwrap().crop_view().is_some());
    assert!(frame.editor.as_ref().unwrap().layers().is_none());
    assert!(frame.image.pixels.same_storage(&raw.pixels));
}

fn verify_pointer_edit(app: &mut Application, raw: &crate::desktop::image::Image) {
    use windows::Win32::{
        Foundation::LPARAM,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    };
    let doc = &app.documents[0];
    let hwnd = doc.window.0;
    doc.state.canvas.borrow_mut().tool = Tool::Brush;
    let viewport = doc.state.canvas.borrow().viewport;
    let dpi = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
    let x = (viewport.origin.x * dpi).round() as i16;
    let y = (viewport.origin.y * dpi).round() as i16;
    let point = LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize);
    unsafe {
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point));
        app.pending_windows(false).unwrap();
    }
    let state = &app.documents[0].state;
    assert!(state.pending_adoption.borrow().is_none());
    assert_eq!(state.canvas.borrow().editor.elements().len(), 1);
    {
        let display = state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert!(frame.image.pixels.same_storage(&raw.pixels));
        assert_eq!(frame.editor.as_ref().unwrap().elements().len(), 1);
        assert!(frame.editor.as_ref().unwrap().layers().is_none());
    }
    assert!(state
        .execution_results
        .borrow()
        .image_source("output")
        .unwrap()
        .image
        .pixels
        .same_storage(&raw.pixels));
    assert!(app.documents[2]
        .state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .editor
        .is_none());
    assert!(state.canvas.borrow_mut().editor.undo());
    app.sync_display_frames();
    let state = &app.documents[0].state;
    state.canvas.borrow_mut().tool = Tool::Eraser;
    let stamp = state.canvas.borrow().editor.document_stamp();
    unsafe {
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point));
    }
    assert!(state.pending_adoption.borrow().is_none());
    assert!(stamp.matches(&state.canvas.borrow().editor));
    assert!(!state.erase_job.borrow().is_busy());
}
