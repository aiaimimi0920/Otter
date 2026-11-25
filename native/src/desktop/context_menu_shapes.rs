//! Modal editing transactions for custom-shape ratios/steps; release Canvas borrows before showing dialogs and write nothing on cancellation.
use crate::desktop::{file_dialog, shape_dialog, State};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindow};

// false means modal admission was denied; callers must skip trailing menu synchronization and preserve the original rejection semantics.
pub(super) unsafe fn edit(hwnd: HWND, state: &State, ratio: bool) -> bool {
    let Some(_dialog) = file_dialog::Guard::begin(&state.file_dialog) else {
        return false;
    };
    let value = {
        let canvas = state.canvas.borrow();
        if ratio {
            canvas.tool_profiles.shapes.ratio_label().to_owned()
        } else {
            canvas.tool_profiles.shapes.step().to_string()
        }
    };
    if let Some(value) = shape_dialog::show(hwnd, value, ratio) {
        if IsWindow(Some(hwnd)).as_bool() {
            let mut canvas = state.canvas.borrow_mut();
            if ratio {
                canvas.tool_profiles.shapes.set_ratio(&value);
            } else {
                canvas.tool_profiles.shapes.set_step(&value);
            }
        }
    }
    true
}
