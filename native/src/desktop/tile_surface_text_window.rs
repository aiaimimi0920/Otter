//! Tile文本编辑接线；Win32调用前移出owner，回调不持RefCell借用，不做网络或阻塞等待。
use super::*;
use crate::{desktop::surface_edit_messages as messages, tile_surface_text::Field};
#[path = "tile_surface_text_owner.rs"]
mod owner;
#[path = "tile_surface_text_popup.rs"]
pub(super) mod popup;
pub(super) use owner::Owner;
pub(super) const DISMISS: u32 = WM_APP + 0x617;
// 显式Esc取消允许保留Tab位置；失焦/外部关闭仍完整退休。
pub(super) const CANCEL_EDIT: u32 = WM_APP + 0x618;
fn current_frame(state: &State) -> Option<Arc<Frame>> {
    actions::active(state)
}
fn field(state: &State, pixel: [i32; 2]) -> Option<Field> {
    let frame = current_frame(state)?;
    state.renderer.borrow().as_ref()?.text(&frame, pixel)
}
fn run(state: &State, f: impl FnOnce(&mut Owner) -> Result<()>) -> bool {
    let Some(mut owner) = state.text.borrow_mut().take() else {
        return true;
    };
    let result = f(&mut owner);
    let succeeded = result.is_ok();
    let previews = owner.previews();
    let old = state.text_preview.borrow().clone();
    *state.text_preview.borrow_mut() = previews;
    if !state.closed.get() && !state.changed.get() && !state.text_retired.get() {
        *state.text.borrow_mut() = Some(owner);
    } else {
        owner.retire(state);
        *state.text_preview.borrow_mut() = Default::default();
    }
    let changed = *old != **state.text_preview.borrow();
    if let Err(error) = result {
        state.shared.lock().actions.notice = "Art文字编辑未提交";
        #[cfg(test)]
        eprintln!("tile-text-edit: {error}");
        let _ = error;
    }
    if changed && !state.closed.get() {
        state.text_dirty.set(true);
    }
    succeeded
}
pub(super) fn begin(hwnd: HWND, state: &State, pixel: [i32; 2]) -> bool {
    let Some(field) = field(state, pixel) else {
        return false;
    };
    state.text_retired.set(false);
    keyboard::close_for_text(state);
    actions::slider::cancel(hwnd, state);
    if state.text_retired.get() || actions::active(state).is_none() {
        return true;
    }
    if state.text.borrow().is_none() {
        match Owner::new(hwnd) {
            Ok(owner) => *state.text.borrow_mut() = Some(owner),
            Err(_) => {
                state.shared.lock().actions.notice = "Art文字编辑器不可用";
                return true;
            }
        }
    }
    if run(state, |owner| owner.open(hwnd, field, state)) && !state.text_retired.get() {
        keyboard::adopt(state, pixel);
    }
    true
}
pub(super) fn tick(state: &State) {
    run(state, |owner| owner.synchronize(state));
}
pub(super) fn commit_for_click(state: &State) -> bool {
    run(state, |owner| {
        if let Some(slot) = owner.active.as_ref().and_then(|id| owner.fields.get(id)) {
            *state.text_click.borrow_mut() =
                Some((slot.field.anchor.view.clone(), slot.edit.clone()));
        }
        owner.close(state, true)
    })
}
pub(super) fn decorate(
    state: &State,
    commands: &mut [crate::tile_surface_actions::Command],
) -> Result<()> {
    if let Some(owner) = state.text.borrow().as_ref() {
        owner.decorate(commands)?;
    }
    Ok(())
}
pub(super) fn peer(
    state: &State,
    view: &crate::tile_surface_wire::View,
    node: String,
    contract: [u8; 32],
) -> Option<Arc<crate::tile_surface_edit::Edit>> {
    state
        .text_click
        .borrow()
        .as_ref()
        .filter(|(v, e)| v == view && e.active())
        .map(|(_, e)| crate::tile_surface_edit::Edit::peer(node, contract, e))
}
pub(super) fn retire(state: &State) {
    state.text_retired.set(true);
    if !state.text_preview.borrow().is_empty() {
        state.text_dirty.set(true);
    }
    let owner = state.text.borrow_mut().take();
    if let Some(mut owner) = owner {
        owner.retire(state);
    }
    *state.text_preview.borrow_mut() = Default::default();
    let dependency = state.text_click.borrow_mut().take();
    if let Some((_, edit)) = dependency {
        state.shared.lock().actions.cancel_edit(&edit);
    }
}
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if matches!(msg, WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC)
        && state
            .text
            .borrow()
            .as_ref()
            .is_some_and(|o| o.owns(HWND(lp.0 as *mut _)).is_some())
    {
        let parent = GetParent(HWND(lp.0 as *mut _)).ok()?;
        return crate::desktop::surface_edit_control::paint(
            parent,
            HWND(lp.0 as *mut _),
            HDC(wp.0 as *mut _),
        )
        .or_else(|| crate::desktop::surface_control_colors::paint(HDC(wp.0 as *mut _)));
    }
    if matches!(msg, DISMISS | CANCEL_EDIT) {
        if state
            .text
            .borrow()
            .as_ref()
            .is_some_and(|o| o.popup.matches(wp, lp))
        {
            if msg == CANCEL_EDIT {
                keyboard::dismiss(hwnd, state);
            } else {
                retire(state);
                keyboard::retire(state);
            }
            return Some(LRESULT(0));
        }
        return None;
    }
    if msg == messages::CAN_PASTE {
        return Some(LRESULT(isize::from(
            state
                .text
                .borrow()
                .as_ref()
                .is_some_and(|o| o.owns(HWND(lp.0 as *mut _)).is_some()),
        )));
    }
    if msg != WM_COMMAND && !(messages::COMPOSITION_START..=messages::ENTER_KEY).contains(&msg) {
        return None;
    }
    let child = HWND(lp.0 as *mut _);
    let known = state.text.borrow().as_ref().and_then(|o| o.owns(child));
    let id = known?;
    if msg == messages::NAVIGATE {
        keyboard::navigate(hwnd, state, wp.0 != 0);
        return Some(LRESULT(0));
    }
    run(state, |owner| {
        let slot = owner.fields.get_mut(&id).unwrap();
        match msg {
            messages::EDIT_BEGIN => {
                if slot.depth == 0 {
                    slot.session.begin_edit()?;
                }
                slot.depth = slot.depth.saturating_add(1);
            }
            messages::EDIT_END => {
                slot.depth = slot.depth.saturating_sub(1);
                if slot.depth == 0 {
                    let changed = slot
                        .session
                        .finish_edit(std::mem::take(&mut slot.notified))?;
                    if changed {
                        slot.capture(false)?;
                        slot.input_changed();
                    }
                }
            }
            messages::COMPOSITION_START => slot.session.begin_composition()?,
            messages::COMPOSITION_END if slot.session.end_composition()? => {
                slot.input_changed();
            }
            messages::STEP_UP | messages::STEP_DOWN
                if slot.session.step_number(msg == messages::STEP_UP)? =>
            {
                slot.input_changed();
                slot.capture(true)?;
            }
            messages::COMMIT => slot.capture(true)?,
            WM_COMMAND => match (wp.0 >> 16) as u32 {
                EN_CHANGE => {
                    if slot.depth > 0 {
                        slot.notified = true;
                    } else {
                        slot.capture(false)?;
                    }
                }
                EN_KILLFOCUS => {}
                _ => {}
            },
            _ => {}
        }
        owner.flush_one(&id, state)
    });
    if !state.closed.get() {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Some(LRESULT(0))
}
#[cfg(test)]
#[path = "tile_surface_text_tests.rs"]
mod tests;
#[cfg(test)]
pub(super) fn active_child(state: &State) -> HWND {
    let owner = state.text.borrow();
    let owner = owner.as_ref().unwrap();
    owner.fields[owner.active.as_ref().unwrap()]
        .session
        .control()
        .hwnd()
        .unwrap()
}
#[cfg(test)]
impl Window {
    pub(crate) fn test_art_text(&self, pixel: [i32; 2], value: &str, ime: bool, commit: bool) {
        assert!(begin(self.hwnd, &self.state, pixel));
        let child = {
            let o = self.state.text.borrow();
            let o = o.as_ref().unwrap();
            o.fields[o.active.as_ref().unwrap()]
                .session
                .control()
                .hwnd()
                .unwrap()
        };
        unsafe {
            use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
            if ime {
                SendMessageW(child, WM_IME_STARTCOMPOSITION, None, None);
            }
            SendMessageW(child, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            let value = windows::core::HSTRING::from(value);
            SendMessageW(
                child,
                EM_REPLACESEL,
                Some(WPARAM(1)),
                Some(LPARAM(value.as_ptr() as isize)),
            );
            if ime {
                SendMessageW(child, WM_IME_ENDCOMPOSITION, None, None);
            }
        }
        if commit {
            assert!(commit_for_click(&self.state));
        }
    }
}
