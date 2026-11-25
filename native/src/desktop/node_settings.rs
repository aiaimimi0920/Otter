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
    pub checked: bool,
    pub action: Action,
}
pub(super) fn label_id(action: Action) -> &'static str {
    match action {
        Action::Endpoints => "node-show-collapsed-endpoints",
        Action::Upstream => "node-run-on-upstream",
        Action::Parameter => "node-run-on-parameter",
        Action::Notify => "node-notify-downstream",
        Action::Relay => "node-accept-upstream-edits",
    }
}
pub(super) fn fields(state: &State) -> Vec<Field> {
    let settings = state.node_settings.get();
    let mut fields = vec![Field {
        checked: !settings.hide_collapsed_endpoints,
        action: Action::Endpoints,
    }];
    let selected = state.selected_art.borrow();
    if let Some(selected) = selected.as_ref() {
        let config = selected.execution;
        fields.extend([
            Field {
                checked: config.accepts_upstream_notification(),
                action: Action::Upstream,
            },
            Field {
                checked: config.param_driven,
                action: Action::Parameter,
            },
            Field {
                checked: config.notify_downstream,
                action: Action::Notify,
            },
        ]);
    } else if !state.is_art() {
        fields.extend([
            Field {
                checked: settings.notify_downstream,
                action: Action::Notify,
            },
            Field {
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
