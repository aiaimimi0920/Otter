//! 显式配置菜单；同步UI只提交单槽意图，Surface/attachment始终锁定本次观看宿主。
use super::*;
use crate::{
    live_relay_trigger_command::Command,
    live_relay_trigger_registration::{Condition, Operator, Target},
    live_relay_viewer_trigger_queue::Phase,
};
use windows::core::HSTRING;
pub(crate) const OPEN: usize = 0x1fa0;
pub(super) struct Menu(pub(super) HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    AppendMenuW(
        GetSystemMenu(hwnd, false),
        MF_STRING | MF_GRAYED,
        OPEN,
        w!("配置触发条件"),
    )
}
fn queue(state: &State) -> Option<Arc<crate::live_relay_control_runtime::interaction::Queue>> {
    state.input.borrow().queue.clone()
}
unsafe fn submit(hwnd: HWND, state: &State, command: Command) -> bool {
    if state.closed.get()
        || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != state as *const State as isize
    {
        return false;
    }
    input::release(hwnd, state);
    let Some(q) = queue(state) else { return false };
    let accepted = q.triggers.submit(command);
    if !accepted {
        q.triggers.reject();
    }
    q.notify_trigger();
    refresh(hwnd, state);
    accepted
}
pub(super) unsafe fn refresh(hwnd: HWND, state: &State) {
    let Some(q) = queue(state) else { return };
    let local = q.local_triggers.host().is_some();
    let phase = if local {
        q.local_triggers.phase()
    } else {
        q.triggers.phase()
    };
    let available = local || q.triggers.available();
    if state.closed.get()
        || state
            .input
            .borrow_mut()
            .trigger_shown
            .replace((phase, available))
            == Some((phase, available))
    {
        return;
    }
    let title = match phase {
        Phase::Idle => "配置触发条件",
        Phase::Pending => "配置触发条件（提交中）",
        Phase::Confirmed => "配置触发条件（已确认）",
        Phase::Rejected => "配置触发条件（已拒绝，可重试）",
        Phase::Unknown => "配置触发条件（结果未知，请重新加入）",
    };
    let _ = ModifyMenuW(
        GetSystemMenu(hwnd, false),
        OPEN as u32,
        MF_BYCOMMAND
            | MF_STRING
            | if available && (local || !matches!(phase, Phase::Pending | Phase::Unknown)) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        OPEN,
        &HSTRING::from(title),
    );
}
fn seed(
    b: &crate::live_relay_viewer_business::Business,
    binding: (String, String),
) -> Option<Command> {
    let observation = b.observations().iter().next()?;
    let index = (1..=65).find(|i| {
        !b.registrations()
            .iter()
            .any(|r| r.binding.binding_id == format!("binding:native:{i}"))
    })?;
    Some(Command {
        binding_id: format!("binding:native:{index}"),
        enabled: true,
        condition: Condition {
            condition_id: format!("condition:native:{index}"),
            revision: 1,
            observation_id: observation.observation_id.clone(),
            operator: Operator::Equals,
            operand: serde_json::json!({"path":"/value/text","value":""}),
            stable_for_ms: 1000,
            rising_edge: true,
            rearm: true,
            minimum_confidence: crate::live_relay_observation::Confidence::Exact,
        },
        target: Target {
            surface_instance_id: binding.0,
            surface_attachment_id: binding.1,
            surface_node_id: "node:choose".into(),
            surface_event: "click".into(),
            surface_action: "action:choose".into(),
        },
    })
}
unsafe fn choose(hwnd: HWND, b: &crate::live_relay_viewer_business::Business) -> Result<usize> {
    let menu = Menu(CreatePopupMenu()?);
    AppendMenuW(
        menu.0,
        MF_STRING
            | if b.observations().iter().next().is_some() && b.registrations().len() < 64 {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        1,
        w!("新增触发条件"),
    )?;
    for (i, r) in b.registrations().iter().enumerate() {
        // wire identifier仅ASCII合法字符，不能用远端文本注入菜单快捷键或换行。
        AppendMenuW(
            menu.0,
            MF_STRING,
            2 + i * 2,
            &HSTRING::from(format!("编辑 {}", r.binding.binding_id)),
        )?;
        AppendMenuW(
            menu.0,
            MF_STRING
                | if r.binding.enabled {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
            3 + i * 2,
            &HSTRING::from(format!("禁用 {}", r.binding.binding_id)),
        )?;
    }
    let mut point = POINT::default();
    GetCursorPos(&mut point)?;
    Ok(TrackPopupMenu(
        menu.0,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        None,
        hwnd,
        None,
    )
    .0 as usize)
}
pub(super) unsafe fn handle(hwnd: HWND, state: &State, message: u32, wp: WPARAM) -> bool {
    if message != WM_SYSCOMMAND || wp.0 & 0xfff0 != OPEN {
        return false;
    }
    if state.closed.get() {
        return true;
    }
    input::release(hwnd, state);
    let Some(q) = queue(state) else { return true };
    if q.local_triggers.host().is_some() {
        super::local_trigger::handle(hwnd, state, &q);
        refresh(hwnd, state);
        return true;
    }
    let Some(binding) = q.triggers.binding() else {
        return true;
    };
    if matches!(q.triggers.phase(), Phase::Pending | Phase::Unknown) {
        return true;
    }
    let b = {
        state
            .host
            .borrow()
            .as_ref()
            .and_then(Host::business_snapshot)
    };
    let Some(b) = b else { return true };
    let selected = choose(hwnd, &b).unwrap_or(0);
    let (draft, disable) = if selected == 1 {
        (seed(&b, binding), false)
    } else if selected >= 2 {
        let disable = selected % 2 == 1;
        (
            b.registrations()
                .get((selected - 2) / 2)
                .and_then(|r| Command::update(r, disable)),
            disable,
        )
    } else {
        (None, false)
    };
    if let Some(draft) = draft {
        let command = if disable {
            Some(draft)
        } else {
            serde_json::to_string_pretty(&draft)
                .ok()
                .and_then(|text| crate::desktop::shape_dialog::show_trigger(hwnd, text))
                .and_then(|text| Command::parse(&text).ok())
                .filter(|command| command.binding_id == draft.binding_id)
        };
        if !state.closed.get()
            && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == state as *const State as isize
        {
            if let Some(command) = command {
                submit(hwnd, state, command);
            }
            refresh(hwnd, state);
        }
    }
    true
}
#[cfg(test)]
pub(crate) unsafe fn submit_test(hwnd: HWND, text: &str) -> bool {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    pointer
        .as_ref()
        .is_some_and(|state| Command::parse(text).is_ok_and(|command| submit(hwnd, state, command)))
}
#[cfg(test)]
pub(crate) unsafe fn phase_test(hwnd: HWND) -> Option<Phase> {
    (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State)
        .as_ref()
        .and_then(queue)
        .map(|q| q.triggers.phase())
}
#[cfg(test)]
pub(crate) unsafe fn local_submit_test(hwnd: HWND, condition: Condition) -> bool {
    let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() else {
        return false;
    };
    if state.closed.get() {
        return false;
    }
    input::release(hwnd, state);
    let Some(q) = queue(state) else { return false };
    let accepted = super::local_trigger::submit(&q, condition);
    refresh(hwnd, state);
    accepted
}
#[cfg(test)]
pub(crate) unsafe fn local_stop_test(hwnd: HWND) -> bool {
    let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() else {
        return false;
    };
    let Some(q) = queue(state) else { return false };
    let accepted = q.local_triggers.stop();
    q.notify_trigger();
    refresh(hwnd, state);
    accepted
}
#[cfg(test)]
pub(crate) unsafe fn local_registration_test(
    hwnd: HWND,
) -> Option<(Phase, crate::art_live_trigger::Scope, Condition, bool)> {
    let state = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref()?;
    let q = queue(state)?;
    let (scope, condition, enabled) = q.local_triggers.registration()?;
    Some((q.local_triggers.phase(), scope, condition, enabled))
}
