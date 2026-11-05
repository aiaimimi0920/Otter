//! 客户区文字/按钮光标使用 Windows 共享资源；与正式命中、输入准入保持一致。
use super::*;
#[derive(Debug, PartialEq, Eq)]
enum Cursor {
    Text,
    Hand,
    Forbidden,
}
fn at(state: &State, point: [i32; 2]) -> Result<Option<Cursor>> {
    let editing = state.local_edit.get();
    let submitting = state.pointer.borrow().enabled();
    if state.closed.get() || (!editing && !submitting) {
        return Ok(None);
    }
    let mut cursor = None;
    if !state.run(|host| {
        cursor = if editing
            && (host.presentation().text_selection_dragging() || host.selectable_text_at(point)?)
        {
            Some(Cursor::Text)
        } else if submitting {
            host.button_enabled_at(point)?.map(|enabled| {
                if enabled {
                    Cursor::Hand
                } else {
                    Cursor::Forbidden
                }
            })
        } else {
            None
        };
        Ok(())
    })? || state.closed.get()
    {
        return Ok(None);
    }
    Ok(cursor)
}

pub(super) unsafe fn set(hwnd: HWND, state: &State, lp: LPARAM) -> bool {
    if state.closed.get() || (lp.0 as u16) != HTCLIENT as u16 {
        return false;
    }
    let mut point = POINT::default();
    if GetCursorPos(&mut point).is_err() || !ScreenToClient(hwnd, &mut point).as_bool() {
        return false;
    }
    let cursor = match at(state, [point.x, point.y]) {
        Ok(Some(cursor)) => cursor,
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            return false;
        }
        _ => return false,
    };
    let Ok(cursor) = LoadCursorW(
        None,
        match cursor {
            Cursor::Text => IDC_IBEAM,
            Cursor::Hand => IDC_HAND,
            Cursor::Forbidden => IDC_NO,
        },
    ) else {
        return false;
    };
    SetCursor(Some(cursor));
    true
}
#[cfg(test)]
#[path = "surface_window_text_cursor_tests.rs"]
mod tests;
