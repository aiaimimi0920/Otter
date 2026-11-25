//! Node-setting actions reuse existing execution/relay owners; window notifications submit local policy only, without network execution.
use super::{execution_settings, State};
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy)]
pub(super) enum Action {
    Endpoints,
    Upstream,
    Parameter,
    Notify,
    Relay,
}
pub(super) struct Field {
    pub label: &'static str,
    pub checked: bool,
    pub action: Action,
}
pub(super) fn fields(state: &State) -> Vec<Field> {
    let settings = state.node_settings.get();
    let mut fields = vec![Field {
        label: "收起时显示端点",
        checked: !settings.hide_collapsed_endpoints,
        action: Action::Endpoints,
    }];
    let selected = state.selected_art.borrow();
    if let Some(selected) = selected.as_ref() {
        let config = selected.execution;
        fields.extend([
            Field {
                label: "上游变化自动执行",
                checked: config.accepts_upstream_notification(),
                action: Action::Upstream,
            },
            Field {
                label: "参数变化自动执行",
                checked: config.param_driven,
                action: Action::Parameter,
            },
            Field {
                label: "通知下游",
                checked: config.notify_downstream,
                action: Action::Notify,
            },
        ]);
    } else if !state.is_art() {
        fields.extend([
            Field {
                label: "通知下游",
                checked: settings.notify_downstream,
                action: Action::Notify,
            },
            Field {
                label: "接受上级贴图编辑传导",
                checked: state.accept_upstream.get(),
                action: Action::Relay,
            },
        ]);
    }
    fields
}
pub(super) unsafe fn apply(hwnd: HWND, state: &State, action: Action) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || state.group_locked.get() || state.session_end.paused() {
        return Ok(());
    }
    match action {
        Action::Upstream => execution_settings::toggle_upstream(hwnd, state),
        Action::Parameter => execution_settings::toggle(hwnd, state, execution_settings::FIRST + 1),
        Action::Notify if state.is_art() => {
            execution_settings::toggle(hwnd, state, execution_settings::FIRST + 3)
        }
        Action::Relay => super::sticker_propagation::toggle(hwnd, state),
        Action::Endpoints | Action::Notify => {
            let before = state.node_settings.get();
            let mut next = before;
            match action {
                Action::Endpoints => next.hide_collapsed_endpoints = !next.hide_collapsed_endpoints,
                _ => next.notify_downstream = !next.notify_downstream,
            }
            state.node_settings.set(next);
            if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
                state.node_settings.set(before);
                return Err(error);
            }
            Ok(())
        }
    }
}
