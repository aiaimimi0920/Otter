//! Tile keyboard entry/focus lifecycle; operate only this owned tree, without input injection or global hooks.
use super::*;
use crate::{
    desktop::surface_edit_messages as messages, surface_scene::Kind, tile_surface_focus::Focus,
};
use std::rc::Rc;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
#[path = "tile_surface_keyboard_value.rs"]
mod value;
pub(super) use value::Owner;
#[derive(Default)]
pub(super) struct Keyboard {
    pub focus: Option<Focus>,
    space: Option<Focus>,
    identity: Rc<()>,
}
pub(super) fn current(state: &State, focus: &Focus) -> Option<Focus> {
    let frame = actions::active(state)?;
    current_in_frame(state, focus, &frame)
}
fn current_in_frame(state: &State, focus: &Focus, frame: &Frame) -> Option<Focus> {
    state
        .renderer
        .borrow()
        .as_ref()?
        .focus_at(frame, focus.anchor.pixel)
        .filter(|f| focus.same(f))
}
fn dirty(state: &State) {
    state.text_dirty.set(true);
}
pub(super) fn cancel_press(state: &State) {
    state.keyboard.borrow_mut().space = None;
    dirty(state);
}
pub(super) fn scrolled(state: &State) {
    *state.keyboard.borrow_mut() = Default::default();
    dirty(state);
}
fn close_value(state: &State, cancel: bool) {
    state.keyboard_retired.set(true);
    let owner = state.keyboard_value.borrow_mut().take();
    if let Some(owner) = owner {
        if cancel {
            state.shared.lock().actions.cancel_edit(&owner.edit);
        } else {
            *state.text_click.borrow_mut() =
                Some((owner.focus.anchor.view.clone(), owner.edit.clone()));
        }
        drop(owner);
    }
}
pub(super) fn close_for_text(state: &State) {
    close_value(state, false);
}
fn run_value(state: &State, f: impl FnOnce(&mut Owner) -> Result<()>) {
    let Some(mut owner) = state.keyboard_value.borrow_mut().take() else {
        return;
    };
    let result = f(&mut owner);
    if result.is_ok()
        && !state.closed.get()
        && !state.changed.get()
        && !state.keyboard_retired.get()
        && owner.edit.active()
    {
        *state.keyboard_value.borrow_mut() = Some(owner);
    } else {
        state.shared.lock().actions.cancel_edit(&owner.edit);
        state.shared.lock().actions.notice = "Art键盘字段未提交";
    }
}
pub(super) fn retire(state: &State) {
    retire_focus(state);
}
fn retire_focus(state: &State) -> Rc<()> {
    // Advance generation before synchronous callbacks such as DestroyWindow; cancellation/navigation/successor edits inside callbacks may permanently veto old restoration.
    let next = Keyboard::default();
    let identity = next.identity.clone();
    *state.keyboard.borrow_mut() = next;
    dirty(state);
    close_value(state, true);
    let dependency = state.text_click.borrow_mut().take();
    if let Some((_, edit)) = dependency {
        state.shared.lock().actions.cancel_edit(&edit);
    }
    identity
}
pub(super) fn dismiss(hwnd: HWND, state: &State) {
    let focus = state
        .keyboard
        .borrow()
        .focus
        .clone()
        .and_then(|f| current(state, &f));
    let identity = retire_focus(state);
    if !Rc::ptr_eq(&identity, &state.keyboard.borrow().identity) {
        return;
    }
    text::retire(state);
    let focus = focus.and_then(|f| {
        current(state, &f).filter(|next| Arc::ptr_eq(&f.anchor.snapshot, &next.anchor.snapshot))
    });
    if !Rc::ptr_eq(&identity, &state.keyboard.borrow().identity) {
        return;
    }
    // Cancel the original Edit and unsent suffix, not the entire Tile or logical Tab position; invalidation/preparation does not restore eligibility.
    state.keyboard.borrow_mut().focus = focus;
    dirty(state);
    let focused = state.keyboard.borrow().focus.is_some();
    if focused && unsafe { GetAncestor(GetForegroundWindow(), GA_ROOTOWNER) == hwnd } {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
    }
}
pub(super) fn visual(state: &State) -> Option<(Focus, bool)> {
    let k = state.keyboard.borrow();
    let focus = k.focus.as_ref()?;
    Some((
        focus.clone(),
        k.space.as_ref().is_some_and(|s| s.same(focus)),
    ))
}
pub(super) fn tick(state: &State) {
    let focused = state.keyboard.borrow().focus.clone();
    if let Some(focus) = focused {
        if let Some(next) = current(state, &focus) {
            let mut k = state.keyboard.borrow_mut();
            if !Arc::ptr_eq(&focus.anchor.snapshot, &next.anchor.snapshot) {
                k.space = None;
                dirty(state);
            }
            k.focus = Some(next);
            drop(k);
            run_value(state, |owner| owner.synchronize(state));
        } else {
            state.keyboard.borrow_mut().space = None;
            if !focus.awaiting(&state.shared.lock(), Instant::now()) {
                retire(state);
            }
        }
    }
}
pub(super) fn pointer(hwnd: HWND, state: &State, pixel: [i32; 2]) {
    close_value(state, false);
    adopt(state, pixel);
    let focused = state.keyboard.borrow().focus.is_some();
    // The system handles ordinary mouse activation; background programmatic messages must not use SetFocus to activate output and steal user foreground.
    let owns_foreground = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOTOWNER) == hwnd };
    if focused && owns_foreground {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
    }
}
pub(super) fn adopt(state: &State, pixel: [i32; 2]) {
    let focus = actions::active(state)
        .and_then(|frame| state.renderer.borrow().as_ref()?.focus_at(&frame, pixel));
    *state.keyboard.borrow_mut() = Keyboard {
        focus,
        ..Default::default()
    };
    dirty(state);
}
pub(super) fn navigate(hwnd: HWND, state: &State, reverse: bool) {
    let identity = Rc::new(());
    state.keyboard.borrow_mut().identity = identity.clone();
    super::scrollbar::cancel(hwnd, state);
    if !navigation_current(state, &identity) {
        return;
    }
    cancel_press(state);
    let current = state.keyboard.borrow().focus.clone();
    let Some(frame) = actions::active(state) else {
        // Share the same-view waiting fence with normal ticks; gaps must not commit, change focus, or destroy old controls.
        if !current
            .as_ref()
            .is_some_and(|f| f.awaiting(&state.shared.lock(), Instant::now()))
        {
            retire(state);
        }
        return;
    };
    let next = state
        .renderer
        .borrow_mut()
        .as_mut()
        .map(|r| r.next_focus(&frame, current.as_ref(), reverse));
    transfer(hwnd, state, next);
}
/// Search is read-only preparation; failure neither destroys current HWND, commits drafts, nor changes logical focus.
fn transfer(hwnd: HWND, state: &State, next: Option<Result<Option<Focus>>>) {
    let identity = state.keyboard.borrow().identity.clone();
    let next = match next {
        Some(Ok(next)) => next,
        Some(Err(_)) => {
            state.shared.lock().actions.notice = "Art键盘可见区域搜索预算不足";
            return;
        }
        None => {
            state.shared.lock().actions.notice = "Art键盘导航不可用";
            return;
        }
    };
    if !text::commit_for_click(state) || !navigation_current(state, &identity) {
        return;
    }
    close_value(state, false);
    // Old-popup destruction may synchronously cancel/start successor navigation; old stacks must not overwrite or retire new owners.
    if !navigation_current(state, &identity) {
        return;
    }
    actions::slider::cancel(hwnd, state);
    if !navigation_current(state, &identity) {
        return;
    }
    if let Some(focus) = &next {
        let installed = super::scroll::install(
            hwnd,
            state,
            &focus.anchor.view.instance_id,
            &focus.anchor.scroll,
            &focus.anchor.snapshot,
        );
        if !navigation_current(state, &identity) {
            return;
        }
        if installed.is_err() {
            retire(state);
            state.shared.lock().actions.notice = "Art键盘显露未完成";
            return;
        }
    }
    // Hide/DestroyWindow/ReleaseCapture may synchronously reenter; never reattach old candidates.
    if actions::active(state).is_none()
        || next.as_ref().is_some_and(|f| current(state, f).is_none())
    {
        retire(state);
        return;
    }
    state.keyboard.borrow_mut().focus = next.clone();
    dirty(state);
    let Some(next) = next else {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
        return;
    };
    if matches!(next.kind, Kind::Input | Kind::Textarea | Kind::Number) {
        text::begin(hwnd, state, next.anchor.pixel);
    } else if matches!(next.kind, Kind::Select | Kind::Slider) {
        state.keyboard_retired.set(false);
        let created = Owner::new(hwnd, state, next);
        if !navigation_current(state, &identity) {
            return;
        }
        match created {
            Ok(Some(owner))
                if !state.closed.get()
                    && !state.changed.get()
                    && !state.keyboard_retired.get()
                    && owner.edit.active()
                    && current(state, &owner.focus).is_some_and(|f| {
                        Arc::ptr_eq(&f.anchor.snapshot, &owner.focus.anchor.snapshot)
                    }) =>
            {
                *state.keyboard_value.borrow_mut() = Some(owner);
            }
            Ok(None) => unsafe {
                let _ = SetFocus(Some(hwnd));
            },
            _ => {
                retire(state);
                state.shared.lock().actions.notice = "Art键盘控件不可用";
                if actions::active(state).is_some() {
                    unsafe {
                        let _ = SetFocus(Some(hwnd));
                    }
                }
            }
        }
    } else {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
    }
}
fn navigation_current(state: &State, identity: &Rc<()>) -> bool {
    !state.closed.get()
        && !state.changed.get()
        && Rc::ptr_eq(identity, &state.keyboard.borrow().identity)
}
unsafe fn activate(hwnd: HWND, state: &State, focus: &Focus) {
    let Some(current) = current(state, focus) else {
        return;
    };
    if current.kind == Kind::Switch {
        if let Some(field) = actions::values::field(state, current.anchor.pixel) {
            actions::values::finish(hwnd, state, field, current.anchor.pixel, |_| Ok(None));
        }
    } else if current.kind == Kind::Button {
        let commands = actions::prepare(state, current.anchor.pixel);
        state.shared.lock().actions.offer(commands, Instant::now());
    }
}
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    let child = HWND(lp.0 as *mut _);
    let owned = state
        .keyboard_value
        .borrow()
        .as_ref()
        .is_some_and(|o| o.child().ok() == Some(child));
    if matches!(msg, text::DISMISS | text::CANCEL_EDIT)
        && state
            .keyboard_value
            .borrow()
            .as_ref()
            .is_some_and(|o| o.dismiss(wp, lp))
    {
        if msg == text::CANCEL_EDIT {
            dismiss(hwnd, state);
        } else {
            text::retire(state);
            retire(state);
        }
        return Some(LRESULT(0));
    }
    if owned && msg == messages::NAVIGATE {
        navigate(hwnd, state, wp.0 != 0);
        return Some(LRESULT(0));
    }
    if owned
        && (msg == WM_COMMAND || (messages::COMPOSITION_START..=messages::ENTER_KEY).contains(&msg))
    {
        run_value(state, |o| o.notify(state, msg, wp));
        return Some(LRESULT(0));
    }
    if !matches!(msg, WM_KEYDOWN | WM_KEYUP | WM_CHAR) || GetFocus() != hwnd {
        return None;
    }
    if !matches!(wp.0, 9 | 13 | 32) {
        return None;
    }
    let modifiers = actions::modifiers();
    if modifiers.alt_key || modifiers.ctrl_key || modifiers.meta_key {
        state.keyboard.borrow_mut().space = None;
        dirty(state);
        return None;
    }
    if msg == WM_CHAR {
        return Some(LRESULT(0));
    }
    if msg == WM_KEYDOWN && lp.0 as usize & (1 << 30) != 0 {
        return Some(LRESULT(0));
    }
    if wp.0 == 9 {
        if msg == WM_KEYDOWN {
            navigate(hwnd, state, modifiers.shift_key);
        }
        return Some(LRESULT(0));
    }
    let focus = state
        .keyboard
        .borrow()
        .focus
        .clone()
        .and_then(|f| current(state, &f));
    if wp.0 == 32 && msg == WM_KEYDOWN {
        state.keyboard.borrow_mut().space = focus;
    } else if let Some(focus) = focus {
        let activate_now = if wp.0 == 13 {
            msg == WM_KEYDOWN && focus.kind == Kind::Button
        } else {
            state.keyboard.borrow_mut().space.take().is_some_and(|old| {
                old.same(&focus) && Arc::ptr_eq(&old.anchor.snapshot, &focus.anchor.snapshot)
            })
        };
        if activate_now {
            activate(hwnd, state, &focus);
        }
    }
    dirty(state);
    Some(LRESULT(0))
}
#[cfg(test)]
#[path = "tile_surface_keyboard_joint_access.rs"]
pub(super) mod joint_access;
#[cfg(test)]
#[path = "tile_surface_keyboard_tests.rs"]
mod tests;
