//! Pin settings and annotations share an undo cursor; record only successful actual-HWND transactions without changing wheel/move history semantics.
use super::State;
use crate::{editor::PinValue, pin_geometry::PinRect};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};

pub(super) unsafe fn frame(hwnd: HWND) -> Result<PinRect> {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r)?;
    Ok(PinRect {
        x: r.left,
        y: r.top,
        width: (r.right - r.left) as u32,
        height: (r.bottom - r.top) as u32,
    })
}
pub(super) unsafe fn resize(hwnd: HWND, state: &State, next: PinRect) -> Result<bool> {
    let before = frame(hwnd)?;
    if before == next {
        return Ok(false);
    }
    apply(hwnd, state, PinValue::Frame(next))?;
    let next = frame(hwnd)?;
    let changed = record(state, PinValue::Frame(before), PinValue::Frame(next));
    Ok(changed)
}
pub(super) unsafe fn opacity(hwnd: HWND, state: &State, value: f32) -> Result<bool> {
    let (mini, before) = {
        let c = state.canvas.borrow();
        (c.minified.is_some(), c.window_opacity)
    };
    if before == value {
        return Ok(false);
    }
    let after = PinValue::Opacity { mini, value };
    apply(hwnd, state, after)?;
    let changed = record(
        state,
        PinValue::Opacity {
            mini,
            value: before,
        },
        after,
    );
    Ok(changed)
}
pub(super) unsafe fn step(hwnd: HWND, state: &State, redo: bool) -> Result<bool> {
    let (changed, effect, selected) = {
        let mut c = state.canvas.borrow_mut();
        if c.cancel_gesture() {
            return Ok(true);
        }
        let selected = c.selected_indices();
        let changed = if redo {
            c.editor.redo()
        } else {
            c.editor.undo()
        };
        let effect = c.editor.take_pin_change();
        c.sync_raster_source();
        (changed, effect, selected)
    };
    if let Some(effect) = effect {
        if let Err(error) = apply(hwnd, state, effect) {
            // Restore the same cursor on failure; the model must not count an unpresented undo as successful.
            let mut c = state.canvas.borrow_mut();
            let restored = if redo {
                c.editor.undo()
            } else {
                c.editor.redo()
            };
            assert!(
                restored,
                "pin history rollback must retain the inverse entry"
            );
            c.editor.take_pin_change();
            refresh_presentation(&mut c, selected);
            return Err(error);
        }
        refresh_presentation(&mut state.canvas.borrow_mut(), selected);
    }
    Ok(changed)
}
pub(super) unsafe fn style(
    hwnd: HWND,
    state: &State,
    next: crate::pin_frame_style::Style,
) -> Result<bool> {
    let before = state.canvas.borrow().frame_style;
    if before == next {
        return Ok(false);
    }
    apply(hwnd, state, PinValue::Style(next))?;
    Ok(record(
        state,
        PinValue::Style(before),
        PinValue::Style(next),
    ))
}
/// The keyboard entry cannot propagate Result; log failures only at this input boundary and keep window-procedure wiring lightweight.
pub(super) unsafe fn shortcut(hwnd: HWND, state: &State, redo: bool) -> bool {
    match step(hwnd, state, redo) {
        Ok(changed) => changed,
        Err(error) => {
            eprintln!("native-pin-history: {error}");
            false
        }
    }
}
fn record(state: &State, before: PinValue, after: PinValue) -> bool {
    let mut c = state.canvas.borrow_mut();
    let selected = c.selected_indices();
    let changed = c.editor.record_pin_change(before, after);
    if changed {
        refresh_presentation(&mut c, selected);
    }
    changed
}
fn refresh_presentation(c: &mut super::canvas::Canvas, selected: Vec<usize>) {
    let revision = c.editor.revision();
    c.selected = selected.first().map(|index| (*index, revision));
    c.multi_selection = (selected.len() > 1).then_some((selected, revision));
    let super::canvas::Canvas {
        minified, editor, ..
    } = c;
    if let Some(view) = minified {
        view.refresh_presentation_stamp(editor);
    }
}
unsafe fn apply(hwnd: HWND, state: &State, value: PinValue) -> Result<()> {
    match value {
        PinValue::Frame(next) => {
            let before = frame(hwnd)?;
            super::pin_window::animation::stop(hwnd, state);
            state.pin_zoom.set(Default::default());
            if !super::pin_window::presentation::apply(hwnd, state, next) {
                if !super::pin_window::presentation::apply(hwnd, state, before) {
                    return Err(Error::new(
                        E_FAIL,
                        "Pin settings resize and geometry rollback failed",
                    ));
                }
                return Err(Error::new(E_FAIL, "Pin settings resize was not committed"));
            }
        }
        PinValue::Opacity { mini, value } => {
            let mut c = state.canvas.borrow_mut();
            if mini {
                c.minified_opacity = value;
            } else {
                c.normal_opacity = value;
            }
            if mini == c.minified.is_some() {
                c.window_opacity = value;
            }
        }
        PinValue::Style(value) => state.canvas.borrow_mut().frame_style = value,
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(())
}
