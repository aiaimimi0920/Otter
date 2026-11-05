//! 已编辑钉图经中继回退到 Art：后台合成上游，目标覆盖层保持独立。
use super::Application;
use crate::{desktop::image::Image, editor::Tool, viewport::Point};

pub(super) fn verify(app: &mut Application) {
    let selected = app.documents[0].state.selected_art.borrow_mut().take();
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(Image {
                width: 8,
                height: 8,
                pixels: vec![255; 256].into(),
            })
            .unwrap();
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 2.0, y: 2.0 })
            .unwrap();
        assert!(canvas.editor.commit());
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
    let source_stamp = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    let output_stamp = app.documents[1]
        .state
        .execution_results
        .borrow()
        .output_stamp();
    app.sync_display_frames();
    wait(app);
    let state = &app.documents[1].state;
    let image = {
        let display = state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert!(frame.has_overlay_owner(&state.unit_id));
        assert_eq!((frame.image.width, frame.image.height), (4, 4));
        assert!(frame.editor.as_ref().unwrap().elements().is_empty());
        assert!(frame
            .image
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel != [255; 4]));
        frame.image.clone()
    };
    state.canvas.borrow_mut().tool = Tool::Brush;
    assert!(crate::desktop::display_edit::prepare(state));
    {
        let mut canvas = state.canvas.borrow_mut();
        assert!(canvas
            .editor
            .layers()
            .unwrap()
            .base
            .same_storage(&image.pixels));
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
            .unwrap();
        assert!(canvas.editor.commit());
    }
    assert!(crate::desktop::display_edit::settle(state));
    app.sync_display_frames();
    let state = &app.documents[1].state;
    {
        let display = state.resolved_display.borrow();
        let frame = display.as_ref().unwrap();
        assert!(
            frame.image.pixels.same_storage(&image.pixels),
            "target edits reuse source composition"
        );
        assert_eq!(frame.editor.as_ref().unwrap().elements().len(), 1);
        assert!(frame.editor.as_ref().unwrap().layers().is_none());
    }
    assert!(source_stamp.matches(&app.documents[0].state.canvas.borrow().editor));
    assert!(output_stamp.matches(&state.execution_results.borrow()));
    assert!(state.canvas.borrow_mut().editor.undo());
    assert!(state.canvas.borrow().editor.layers().is_none());
    // 源撤销裁剪产生新一代合成，旧像素仍可由在用快照持有，但不得重新发布。
    assert!(app.documents[0].state.canvas.borrow_mut().editor.undo());
    app.sync_display_frames();
    wait(app);
    assert_eq!(
        app.documents[1]
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .image
            .width,
        8
    );
    *app.documents[0].state.selected_art.borrow_mut() = selected;
    app.sync_display_frames();
}

fn wait(app: &mut Application) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.display_cache.borrow().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "display composition timeout"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
        unsafe {
            app.pending_windows(false).unwrap();
        }
    }
}
