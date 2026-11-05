//! 原观看系统菜单中的本地Art自动授权入口；模态返回后复核同一窗口和完整宿主。
use super::*;
use crate::{
    live_relay_trigger_registration::{Condition, Operator},
    live_relay_viewer_trigger_queue::Phase,
};
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    q: &Arc<crate::live_relay_control_runtime::interaction::Queue>,
) {
    let Some(host) = q.local_triggers.host() else {
        return;
    };
    let business = state
        .host
        .borrow()
        .as_ref()
        .and_then(Host::business_snapshot);
    let Some(business) = business else {
        return;
    };
    let current = q.local_triggers.registration();
    let menu = trigger::Menu(match CreatePopupMenu() {
        Ok(menu) => menu,
        Err(_) => return,
    });
    let blocked = matches!(q.local_triggers.phase(), Phase::Pending | Phase::Unknown);
    let _ = AppendMenuW(
        menu.0,
        MF_STRING
            | if !blocked && business.observations().iter().next().is_some() {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        1,
        w!("启用或修改 Art 自动触发…"),
    );
    let _ = AppendMenuW(
        menu.0,
        MF_STRING
            | if current.as_ref().is_some_and(|r| r.2) || q.local_triggers.phase() == Phase::Pending
            {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        2,
        w!("停止 Art 自动触发"),
    );
    let mut point = POINT::default();
    if GetCursorPos(&mut point).is_err() {
        return;
    }
    let chosen = TrackPopupMenu(
        menu.0,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        None,
        hwnd,
        None,
    )
    .0;
    if !owns(hwnd, state) {
        return;
    }
    if chosen == 2 {
        q.local_triggers.stop();
        q.notify_trigger();
        return;
    }
    if chosen != 1 || blocked {
        return;
    }
    let condition = if let Some((_, mut c, _)) = current {
        let Some(revision) = c.revision.checked_add(1) else {
            return;
        };
        c.revision = revision;
        c
    } else {
        let Some(observation) = business.observations().iter().next() else {
            return;
        };
        Condition {
            condition_id: "condition:native:local".into(),
            revision: 1,
            observation_id: observation.observation_id.clone(),
            operator: Operator::Equals,
            operand: serde_json::json!({"path":"/value/text","value":""}),
            stable_for_ms: 1000,
            rising_edge: true,
            rearm: true,
            minimum_confidence: crate::live_relay_observation::Confidence::Exact,
        }
    };
    let Some(text) = serde_json::to_string_pretty(&condition)
        .ok()
        .and_then(|text| crate::desktop::shape_dialog::show_local_trigger(hwnd, text))
    else {
        return;
    };
    let Ok(parsed) = serde_json::from_str::<Condition>(&text) else {
        return;
    };
    if parsed.revision != condition.revision
        || parsed.condition_id != condition.condition_id
        || !owns(hwnd, state)
        || q.local_triggers
            .host()
            .is_none_or(|actual| actual.binding != host.binding || actual.identity != host.identity)
    {
        return;
    }
    submit(q, parsed);
}
fn owns(hwnd: HWND, state: &State) -> bool {
    !state.closed.get()
        && unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) == state as *const State as isize }
}
pub(super) fn submit(
    q: &Arc<crate::live_relay_control_runtime::interaction::Queue>,
    condition: Condition,
) -> bool {
    let Ok(id) = crate::desktop::document_state::new_unit_id() else {
        return false;
    };
    let accepted = q.local_triggers.submit(condition, id.to_string());
    q.notify_trigger();
    accepted
}
