//! Tile native clicks and explicit right-click operations; no WndProc networking/automatic confirmation or shared locks across menu/modal pumps.
use super::*;
use crate::{
    surface_confirmation::{Choice, Risk},
    tile_surface_actions::{Anchor, Command, Kind},
};
use std::time::Duration;
#[path = "tile_surface_slider_window.rs"]
pub(super) mod slider;
#[path = "tile_surface_value_window.rs"]
pub(super) mod values;
#[derive(Default)]
pub(super) enum Gesture {
    #[default]
    None,
    Click(Vec<Command>),
    Value(Box<crate::tile_surface_value::Field>),
    Slider(Box<crate::tile_surface_slider::Drag>),
}
pub(super) fn active(state: &State) -> Option<Arc<Frame>> {
    let s = state.shared.lock();
    (!state.closed.get()
        && !state.changed.get()
        && state.painted.get()
        && !s.frozen
        && s.visible(Instant::now()))
    .then(|| s.presented_frame.clone())
    .flatten()
}
pub(super) fn modifiers() -> crate::surface_click_event::Modifiers {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let pressed = |vk: VIRTUAL_KEY| unsafe { GetKeyState(vk.0 as i32) < 0 };
    crate::surface_click_event::Modifiers {
        alt_key: pressed(VK_MENU),
        ctrl_key: pressed(VK_CONTROL),
        shift_key: pressed(VK_SHIFT),
        meta_key: pressed(VK_LWIN) || pressed(VK_RWIN),
    }
}
pub(super) fn prepare(state: &State, pixel: [i32; 2]) -> Vec<Command> {
    let Some(frame) = active(state) else {
        return Vec::new();
    };
    let result = state
        .renderer
        .borrow()
        .as_ref()
        .map(|r| r.click(&frame, pixel, modifiers()));
    let Some(result) = result else {
        return Vec::new();
    };
    let mut s = state.shared.lock();
    if s.frozen
        || !s.visible(Instant::now())
        || s.presented_frame
            .as_ref()
            .is_none_or(|f| f.key != frame.key)
    {
        return Vec::new();
    }
    match result {
        Ok(mut commands) => {
            if super::text::decorate(state, &mut commands).is_err() {
                s.actions.notice = "integration-art-form-not-committed";
                return Vec::new();
            }
            if let Some((view, edit)) = state
                .text_click
                .borrow()
                .as_ref()
                .filter(|(_, e)| e.active())
            {
                for command in &mut commands {
                    if command.anchor.view == *view {
                        command.edit = Some(edit.clone());
                    }
                }
            }
            commands
        }
        Err(_) => {
            s.actions.notice = "integration-art-click-uncommitted";
            Vec::new()
        }
    }
}
pub(super) fn down(state: &State, pixel: [i32; 2]) {
    let gesture = if let Some(value) = values::field(state, pixel) {
        Gesture::Value(Box::new(value))
    } else {
        Gesture::Click(prepare(state, pixel))
    };
    *state.pressed.borrow_mut() = gesture;
}
/// The same press edge commits old edits before taking over new fields/gestures; no owner borrow crosses Win32 callbacks.
pub(super) fn pointer_down(hwnd: HWND, state: &State, pixel: [i32; 2]) {
    if super::text::begin(hwnd, state, pixel) || !super::text::commit_for_click(state) {
        return;
    }
    super::keyboard::pointer(hwnd, state, pixel);
    if !slider::begin(hwnd, state, pixel) {
        slider::cancel(hwnd, state);
        down(state, pixel);
    }
}
pub(super) fn click(hwnd: HWND, state: &State, pixel: [i32; 2]) {
    let prior = std::mem::take(&mut *state.pressed.borrow_mut());
    let prior = match prior {
        Gesture::None => return,
        Gesture::Value(value) => {
            values::finish(hwnd, state, *value, pixel, |field| {
                values::choose(hwnd, field)
            });
            return;
        }
        Gesture::Slider(drag) => {
            slider::finish(hwnd, state, *drag, pixel);
            return;
        }
        Gesture::Click(commands) => commands,
    };
    if prior.is_empty() {
        return;
    }
    let commands = prepare(state, pixel);
    if commands.len() != prior.len() || !commands.iter().zip(&prior).all(|(a,b)| a.anchor.view == b.anchor.view && a.anchor.generation == b.anchor.generation && a.anchor.placement == b.anchor.placement && Arc::ptr_eq(&a.anchor.snapshot,&b.anchor.snapshot)
        && matches!((&a.kind,&b.kind),(Kind::Event(a),Kind::Event(b)) if a["nodeId"] == b["nodeId"] && a["action"] == b["action"])) { return; }
    state.shared.lock().actions.offer(commands, Instant::now());
}
fn choices(state: &State) -> Vec<(Command, String)> {
    const LIMIT: usize = 64;
    let s = state.shared.lock();
    if state.closed.get() || state.changed.get() || s.stopped || s.expired {
        return Vec::new();
    }
    let mut result = Vec::new();
    let now = Instant::now();
    let now_ms = crate::tile_runtime::now_ms();
    let can_review = !s.frozen && s.visible(now);
    for current in s.actions.states.values() {
        if current.view.binding.endpoint_id != s.desired.as_ref().map_or("", |k| &k.endpoint)
            || current.view.binding.lease_id != s.desired.as_ref().map_or("", |k| &k.lease)
        {
            continue;
        }
        if can_review {
            for c in &current.activity.confirmations {
                if let Some(a) = s.actions.owns_confirmation(c, current, now_ms) {
                    result.push((
                        Command {
                            edit: None,
                            anchor: a.anchor.clone(),
                            kind: Kind::Decision {
                                confirmation: Box::new(c.clone()),
                                approved: false,
                            },
                            until: now + Duration::from_millis(1500),
                        },
                        c.action_id.clone(),
                    ));
                    if result.len() == LIMIT {
                        return result;
                    }
                }
            }
        }
        for pending in current.activity.pending.iter().filter(|p| p.cancelable) {
            result.push((
                Command {
                    edit: None,
                    anchor: Anchor {
                        view: current.view.clone(),
                        generation: current.generation,
                        snapshot: current.snapshot.clone(),
                        placement: String::new(),
                        pixel: [0, 0],
                        scroll: Default::default(),
                    },
                    kind: Kind::Cancel {
                        request: pending.ack.request_id().into(),
                        event: pending.event.clone(),
                    },
                    until: now + Duration::from_millis(1500),
                },
                pending.action.clone(),
            ));
            if result.len() == LIMIT {
                return result;
            }
        }
    }
    result
}
pub(super) unsafe fn menu(hwnd: HWND, state: &State, point: POINT) {
    if !IsWindowVisible(hwnd).as_bool() || state.shared.lock().identification.blocked() {
        return;
    }
    let options = choices(state);
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    struct Menu(HMENU);
    impl Drop for Menu {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyMenu(self.0);
            }
        }
    }
    let _menu = Menu(menu);
    let notice = state.shared.lock().actions.notice;
    let label = windows::core::HSTRING::from(if notice.is_empty() {
        crate::ui_text::text("integration-art-action")
    } else {
        crate::tile_shared::display_status(notice)
    });
    if AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, &label).is_err() {
        return;
    }
    for (i, (option, action)) in options.iter().enumerate() {
        let action = action.replace('&', "&&");
        let label = match option.kind {
            Kind::Decision { .. } => crate::ui_text::format(
                "integration-review-action",
                &[("action", action.as_str().into())],
            ),
            _ => crate::ui_text::format(
                "integration-cancel-action",
                &[("action", action.as_str().into())],
            ),
        };
        if AppendMenuW(menu, MF_STRING, i + 1, &windows::core::HSTRING::from(label)).is_err() {
            return;
        }
    }
    let index = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        None,
        hwnd,
        None,
    )
    .0 as usize;
    if index == 0 || state.closed.get() || state.changed.get() || !IsWindowVisible(hwnd).as_bool() {
        return;
    }
    if let Some((command, _)) = options.into_iter().nth(index - 1) {
        choose(hwnd, state, command, |origin, c| {
            let risk = match c.risk {
                Risk::Low => crate::ui_text::text("integration-risk-low"),
                Risk::Medium => crate::ui_text::text("integration-risk-medium"),
                Risk::High => crate::ui_text::text("integration-risk-high"),
            };
            let payload = c.payload.as_ref().map(serde_json::Value::to_string);
            let text = crate::desktop::surface_authorized_window::confirmation_dialog::dialog_text(
                origin,
                &c.action_id,
                risk,
                payload.as_deref(),
            )?;
            crate::desktop::surface_authorized_window::confirmation_dialog::native_choice(
                hwnd, &text,
            )
        });
    }
}
fn current_prompt(
    s: &crate::tile_shared::State,
    confirmation: &crate::tile_surface_activity::Confirmation,
) -> bool {
    !s.frozen
        && s.visible(Instant::now())
        && s.actions
            .states
            .get(&confirmation.instance_id)
            .is_some_and(|current| {
                current.activity.confirmations.contains(confirmation)
                    && s.actions
                        .owns_confirmation(confirmation, current, crate::tile_runtime::now_ms())
                        .is_some()
            })
}
fn choose(
    hwnd: HWND,
    state: &State,
    mut command: Command,
    prompt: impl FnOnce(&str, &crate::tile_surface_activity::Confirmation) -> Result<Option<Choice>>,
) {
    if let Kind::Decision {
        confirmation,
        approved,
    } = &mut command.kind
    {
        let origin = {
            let s = state.shared.lock();
            if !current_prompt(&s, confirmation) {
                return;
            }
            s.actions.origin.clone()
        };
        match prompt(&origin, confirmation) {
            Ok(Some(Choice::Approve)) => *approved = true,
            Ok(Some(Choice::Reject)) => *approved = false,
            _ => return,
        }
    }
    if state.closed.get() || state.changed.get() || !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return;
    }
    if let Kind::Decision { confirmation, .. } = &command.kind {
        let s = state.shared.lock();
        if !current_prompt(&s, confirmation) {
            return;
        }
    }
    command.until = Instant::now() + Duration::from_millis(1500);
    state
        .shared
        .lock()
        .actions
        .offer(vec![command], Instant::now());
}
#[cfg(test)]
pub(super) fn test_value(
    hwnd: HWND,
    state: &State,
    pixel: [i32; 2],
    selected: Option<usize>,
) -> bool {
    values::test_value(hwnd, state, pixel, selected)
}
#[cfg(test)]
pub(super) fn test_choice(hwnd: HWND, state: &State, choice: Option<Choice>) -> bool {
    let Some((command, _)) = choices(state).into_iter().next() else {
        return false;
    };
    choose(hwnd, state, command, |_, _| Ok(choice));
    true
}
#[cfg(test)]
#[path = "tile_surface_action_window_tests.rs"]
mod tests;
