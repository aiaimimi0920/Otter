//! 历史文字使用当前显示快照，不读取系统历史或启动窗口。
use super::*;
use crate::{
    desktop::{display_edit, display_frame::Frame, image::Image},
    editor::{Editor, Tool},
    erase_layers::Layers,
};

fn fixture(base: bool) -> State {
    let state = State::new(None, false).unwrap();
    {
        let mut canvas = state.canvas.borrow_mut();
        // 历史菜单不要求用户先切换到文字工具。
        canvas.tool = Tool::Pan;
        if base {
            canvas.editor = Editor::with_layers(Layers {
                size: (80, 80),
                base: vec![255; 80 * 80 * 4].into(),
                annotations: None,
                annotation_size: None,
            })
            .unwrap();
            canvas.sync_raster_source();
        }
    }
    *state.resolved_display.borrow_mut() = Some(Frame::capture(
        crate::desktop::document_state::new_unit_id().unwrap(),
        Image {
            width: 8,
            height: 4,
            pixels: vec![255; 8 * 4 * 4].into(),
        },
        None,
    ));
    state
}

#[test]
fn history_text_adopts_visible_frame_even_with_pan_tool_and_different_or_empty_base() {
    for base in [false, true] {
        let state = fixture(base);
        assert!(import_text(&state, "visible").unwrap());
        assert!(state.resolved_display.borrow().is_none());
        {
            let canvas = state.canvas.borrow();
            let image = canvas.image.as_ref().unwrap();
            assert_eq!((image.width, image.height), (8, 4));
            let row = &canvas.editor.elements()[0];
            assert_eq!(row.text.as_ref().unwrap().content(), "visible");
            assert!(((row.points[0].x + row.points[1].x) / 2.0 - 4.0).abs() < 0.01);
            assert!(((row.points[0].y + row.points[1].y) / 2.0 - 2.0).abs() < 0.01);
        }
        assert!(display_edit::settle(&state));
        assert!(state.pending_adoption.borrow().is_none());
        let mut canvas = state.canvas.borrow_mut();
        assert!(canvas.editor.undo());
        assert!(canvas.editor.elements().is_empty());
        canvas.sync_raster_source();
        assert_eq!(
            canvas
                .image
                .as_ref()
                .map(|image| (image.width, image.height)),
            base.then_some((80, 80))
        );
    }
}

#[test]
fn empty_invalid_and_locked_imports_preserve_unadopted_display() {
    let state = fixture(false);
    let identity = state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .identity
        .clone();
    assert!(!import_text(&state, " \n\t").unwrap());
    assert!(import_text(&state, "bad\0text").is_err());
    state.group_locked.set(true);
    assert!(!import_text(&state, "blocked").unwrap());
    assert!(state.pending_adoption.borrow().is_none());
    assert!(state.canvas.borrow().image.is_none());
    assert!(std::sync::Arc::ptr_eq(
        &identity,
        &state.resolved_display.borrow().as_ref().unwrap().identity
    ));
}
