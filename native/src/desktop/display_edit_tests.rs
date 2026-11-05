//! 文字入口准入回归；坐标位于图片外，不打开模态框或操作用户窗口。
use super::*;
use crate::viewport::Point;
use windows::Win32::Foundation::HWND;

fn state() -> State {
    let state = State::new(None, false).unwrap();
    state.canvas.borrow_mut().tool = Tool::Text;
    *state.resolved_display.borrow_mut() = Some(crate::desktop::display_frame::Frame::capture(
        crate::desktop::document_state::new_unit_id().unwrap(),
        Image {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3, 255].into(),
        },
        None,
    ));
    state
}

#[test]
fn text_admission_rejects_fractional_padding_without_adopting_external_display() {
    let state = state();
    let image = Image {
        width: 8,
        height: 4,
        pixels: vec![255; 128].into(),
    };
    let mut editor = Editor::with_layers(Layers {
        size: (8, 4),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_crop(crate::crop_geometry::Crop {
        rect: crate::crop_geometry::Rect {
            x: 1.0,
            y: 0.5,
            width: 2.0,
            height: 1.0,
        },
        source: crate::crop_geometry::Size {
            width: 7.25,
            height: 3.25,
        },
    })
    .unwrap();
    editor.reset_crop();
    let mut display = state.resolved_display.borrow_mut();
    let frame = display.as_mut().unwrap();
    frame.image = image;
    frame.editor = Some(Box::new(editor));
    drop(display);
    state.canvas.borrow_mut().viewport.origin = Point::default();
    state.canvas.borrow_mut().viewport.scale = 1.0;
    for point in [Point { x: 7.5, y: 1.0 }, Point { x: 1.0, y: 3.5 }] {
        assert!(!prepare_text_at(&state, point));
        assert!(state.pending_adoption.borrow().is_none());
        assert!(state.resolved_display.borrow().is_some());
    }
}

#[test]
fn settlement_waits_for_modal_or_draft_and_cancel_restores_display_identity() {
    let state = state();
    let original = state.canvas.borrow().editor.document_stamp();
    let identity = state
        .resolved_display
        .borrow()
        .as_ref()
        .unwrap()
        .identity
        .clone();
    assert!(prepare(&state));
    state.file_dialog.set(true);
    assert!(!settle(&state));
    state.file_dialog.set(false);
    state
        .canvas
        .borrow_mut()
        .editor
        .begin(Tool::Brush, Point::default())
        .unwrap();
    assert!(!settle(&state));
    state.canvas.borrow_mut().editor.cancel();
    assert!(settle(&state));
    assert!(state.pending_adoption.borrow().is_none());
    assert!(original.matches(&state.canvas.borrow().editor));
    assert!(std::sync::Arc::ptr_eq(
        &identity,
        &state.resolved_display.borrow().as_ref().unwrap().identity
    ));
}

#[test]
fn selected_source_annotation_keeps_provisional_document_until_deselection() {
    let state = state();
    {
        let mut frame = state.resolved_display.borrow_mut();
        let frame = frame.as_mut().unwrap();
        let mut editor = Editor::with_layers(Layers {
            size: (1, 1),
            base: frame.image.pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap();
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(editor.commit());
        frame.editor = Some(Box::new(editor));
    }
    state.canvas.borrow_mut().tool = Tool::Select;
    assert!(prepare(&state));
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas.selected = Some((0, canvas.editor.revision()));
    }
    assert!(!settle(&state));
    state.canvas.borrow_mut().selected = None;
    assert!(settle(&state));
    assert!(state.resolved_display.borrow().is_some());
}

#[test]
fn committed_group_transform_rebinds_every_selected_index() {
    let state = state();
    {
        let mut display = state.resolved_display.borrow_mut();
        let frame = display.as_mut().unwrap();
        let mut editor = Editor::with_layers(Layers {
            size: (1, 1),
            base: frame.image.pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap();
        for x in [0.0, 0.5] {
            editor.begin(Tool::Brush, Point { x, y: 0.0 }).unwrap();
            assert!(editor.commit());
        }
        frame.editor = Some(Box::new(editor));
    }
    state.canvas.borrow_mut().tool = Tool::Select;
    assert!(prepare(&state));
    {
        let mut canvas = state.canvas.borrow_mut();
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        assert!(canvas.resize_selected_axes(Point { x: 2.0, y: 2.0 }));
    }
    assert!(settle(&state));
    assert_eq!(state.canvas.borrow().selected_indices(), [0, 1]);
    assert!(state.canvas.borrow_mut().editor.undo());
    assert!(state.canvas.borrow().editor.layers().is_none());
}

#[test]
fn text_insert_double_click_and_selected_edit_preserve_blocked_display() {
    for locked in [false, true] {
        for command in 0..3 {
            let state = state();
            state.group_locked.set(locked);
            state.file_dialog.set(!locked);
            assert!(!prepare_text_at(&state, Point { x: 20.5, y: 20.5 }));
            let stamp = state.canvas.borrow().editor.document_stamp();
            let identity = state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .identity
                .clone();
            unsafe {
                let outside = Point {
                    x: -100.0,
                    y: -100.0,
                };
                match command {
                    0 => crate::desktop::text_input::insert(HWND::default(), &state, outside),
                    1 => crate::desktop::text_input::edit_at(HWND::default(), &state, outside),
                    _ => crate::desktop::text_input::edit_selected(HWND::default(), &state),
                }
            }
            assert!(stamp.matches(&state.canvas.borrow().editor));
            assert!(state.canvas.borrow().image.is_none());
            assert!(std::sync::Arc::ptr_eq(
                &identity,
                &state.resolved_display.borrow().as_ref().unwrap().identity
            ));
        }
    }
}

#[test]
fn text_outside_display_preserves_history_and_inside_can_adopt() {
    let state = state();
    unsafe {
        crate::desktop::text_input::edit_at(
            HWND::default(),
            &state,
            Point {
                x: -100.0,
                y: -100.0,
            },
        );
    }
    assert!(state.resolved_display.borrow().is_some());
    assert!(!state.canvas.borrow().editor.has_edit_state());
    assert!(prepare_text_at(&state, Point { x: 20.5, y: 20.5 }));
    assert!(state.resolved_display.borrow().is_none());
    let canvas = state.canvas.borrow();
    assert_eq!(&*canvas.image.as_ref().unwrap().pixels, &[1, 2, 3, 255]);
    drop(canvas);
    assert!(settle(&state));
    assert!(state.canvas.borrow().image.is_none());
    assert!(state.resolved_display.borrow().is_some());
}
