//! Business regressions for exact RGBA, single-selection history, and palette deletion; no screen sampling or clipboard access.
use super::*;
use crate::{editor::Tool, stroke_style::PaintAlpha, viewport::Point};
fn state() -> State {
    let state = State::new(None, false).unwrap();
    state.canvas.borrow_mut().select_tool(Tool::Rectangle);
    state
}
#[test]
fn original_picker_svg_icons_are_valid() {
    let i = icons();
    for icon in [i.eyedropper, i.remove, i.apply] {
        icon.validate().unwrap();
    }
}
#[test]
fn rgba_apply_preserves_other_slot_opacity_and_initial_exactly() {
    let s = state();
    let mut c = s.canvas.borrow_mut();
    let stamp = c.editor.document_stamp();
    let selected = Rgba::parse("#10203080").unwrap();
    apply::rgba(&mut c, true, selected);
    assert_eq!(initial(c.editor.style(), true), selected);
    apply::rgba(&mut c, false, Rgba::parse("#a0b0c04d").unwrap());
    assert_eq!(initial(c.editor.style(), false).a, 77);
    assert_eq!(initial(c.editor.style(), true), selected);
    assert_eq!(c.editor.style().opacity(), 1.0);
    assert!(stamp.matches(&c.editor));
}
#[test]
fn selected_rgba_is_one_undo_transaction_not_a_creation_default() {
    let s = state();
    let mut c = s.canvas.borrow_mut();
    c.editor
        .begin(Tool::Line, Point { x: 1.0, y: 1.0 })
        .unwrap();
    c.editor.extend(Point { x: 20.0, y: 20.0 }).unwrap();
    assert!(c.editor.commit());
    c.select_tool(Tool::Select);
    c.selected = Some((0, c.editor.revision()));
    let defaults = c.editor.style();
    let old = c.editor.elements()[0].style();
    let rgba = Rgba::parse("#12345680").unwrap();
    apply::rgba(&mut c, false, rgba);
    assert_eq!(initial(c.editor.elements()[0].style(), false), rgba);
    assert_eq!(c.editor.style(), defaults);
    assert_eq!(c.selected_index(), Some(0));
    assert!(c.editor.undo());
    assert_eq!(c.editor.elements()[0].style(), old);
    assert!(c.editor.redo());
    assert_eq!(initial(c.editor.elements()[0].style(), false), rgba);
}
#[test]
fn removing_palette_color_resets_all_defaults_but_not_selected_elements() {
    let s = state();
    let removed = Rgba::parse("#10203080").unwrap();
    let mut c = s.canvas.borrow_mut();
    apply::rgba(&mut c, false, removed);
    c.select_tool(Tool::Ellipse);
    apply::rgba(&mut c, true, removed);
    c.select_tool(Tool::Serial);
    apply::rgba(&mut c, true, removed);
    c.editor
        .insert_serial(Point { x: 30.0, y: 30.0 }, 14.0, "微软雅黑")
        .unwrap();
    let old = c.editor.elements()[0].style();
    c.select_tool(Tool::Select);
    c.selected = Some((0, c.editor.revision()));
    let stamp = c.editor.document_stamp();
    super::super::super::edit_palette::reset(&mut c, &[removed]);
    assert_eq!(c.editor.elements()[0].style(), old);
    assert!(stamp.matches(&c.editor));
    c.select_tool(Tool::Rectangle);
    assert_eq!(c.editor.style().color(), StrokeStyle::default().color());
    assert_eq!(c.editor.style().paint_alpha().stroke(), 1.0);
    c.select_tool(Tool::Ellipse);
    assert_eq!(c.editor.style().fill(), None);
    c.select_tool(Tool::Serial);
    assert_eq!(c.editor.style().fill(), Some(Color(0, 0, 0)));
}
#[test]
fn palette_delete_can_cancel_and_same_color_apply_after_delete_wins() {
    let s = state();
    let mut c = s.canvas.borrow_mut();
    let selected = Rgba::parse("#1234564d").unwrap();
    apply::rgba(&mut c, false, selected);
    let style = c
        .editor
        .style()
        .with_paint_alpha(PaintAlpha::new(0.3, 0.8).unwrap());
    c.editor.set_style(style);
    drop(c);
    let palette = s.edit_palette.borrow().colors.clone();
    s.edit_palette
        .borrow_mut()
        .record(&s, palette, vec![selected]);
    assert_eq!(
        s.canvas.borrow().editor.style().color(),
        StrokeStyle::default().color()
    );
    apply::rgba(&mut s.canvas.borrow_mut(), false, selected);
    let pending = s.edit_palette.borrow_mut().take_reset().unwrap();
    assert_eq!(pending.origin, s.unit_id);
    assert_eq!(initial(s.canvas.borrow().editor.style(), false), selected);
    assert_eq!(s.canvas.borrow().editor.style().paint_alpha().fill(), 0.8);
}
