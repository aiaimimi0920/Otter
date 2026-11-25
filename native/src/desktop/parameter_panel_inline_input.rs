//! Inline right-click restore/disable and wheel routing; the subclass dies with its actual child and retains no raw State pointer.
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const SUBCLASS: usize = 0x4f545249;
pub(super) unsafe fn install(child: HWND) -> Result<()> {
    SetWindowSubclass(child, Some(procedure), SUBCLASS, 0).ok()
}
pub(super) unsafe fn context(ui: &Ui, id: &str, value: bool) {
    if !ui.available() {
        return;
    }
    let model = ui.model.borrow().clone();
    let Some(row) = model.rows.iter().find(|row| row.id == id) else {
        return;
    };
    if row.widget == "sticker-image" {
        ui.queue(Intent::Sticker(
            model.stamp.clone(),
            if value {
                desktop::sticker_input::Action::Clear
            } else {
                desktop::sticker_input::Action::ToggleDisabled
            },
        ));
    } else if let Some(snapshot) = &model.snapshot {
        let action = if !value {
            row.disable
        } else if row.widget == "color" {
            row.primary.get(1).copied()
        } else {
            row.reset
        };
        if let Some(index) = action {
            ui.queue(Intent::Choice(snapshot.clone(), index, None));
        }
    }
}
pub(super) unsafe fn reveal(ui: &Ui, child: HWND) {
    let bindings = ui.bindings.borrow();
    let Some(binding) = bindings.iter().find(|b| b.hwnd == child) else {
        return;
    };
    let model = ui.model.borrow();
    let id = model
        .rows
        .iter()
        .filter(|r| binding.key.starts_with(&format!("{}:", r.id)))
        .max_by_key(|r| r.id.len())
        .map(|r| r.id.clone());
    drop(model);
    drop(bindings);
    if let (Some(id), Some(state)) = (id, ui.state()) {
        desktop::application::chrome::reveal_parameter(&state.chrome, ui.host, &id);
    }
}
pub(super) unsafe fn leave(ui: &Ui, child: HWND, key: usize) {
    let binding = ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.hwnd == child)
        .cloned();
    let model = ui.model.borrow().clone();
    let id = model
        .rows
        .iter()
        .filter(|r| {
            binding
                .as_ref()
                .is_some_and(|b| b.key.starts_with(&format!("{}:", r.id)))
        })
        .max_by_key(|r| r.id.len())
        .map(|r| r.id.clone());
    if let (Some(id), Some(state)) = (id, ui.state()) {
        desktop::application::chrome::focus_parameter(&state.chrome, ui.host, &id, key);
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS);
    }
    let parent = GetParent(hwnd).unwrap_or_default();
    if msg == WM_MOUSEWHEEL {
        // An open system list consumes its own wheel events; other controls scroll only the parameter rail without accidentally changing fields.
        if SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0 != 1 {
            return SendMessageW(parent, msg, Some(wp), Some(lp));
        }
    }
    if msg == WM_CONTEXTMENU {
        let pointer = GetWindowLongPtrW(parent, GWLP_USERDATA) as *const Ui;
        if !pointer.is_null() {
            let ui = &*pointer;
            let key = ui
                .bindings
                .borrow()
                .iter()
                .find(|b| b.hwnd == hwnd)
                .map(|b| b.key.clone());
            let model = ui.model.borrow().clone();
            let id = model
                .rows
                .iter()
                .filter(|r| {
                    key.as_ref()
                        .is_some_and(|k| k.starts_with(&format!("{}:", r.id)))
                })
                .max_by_key(|r| r.id.len())
                .map(|r| r.id.clone());
            if let Some(id) = id {
                context(ui, &id, true);
            }
        }
        return LRESULT(0);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
