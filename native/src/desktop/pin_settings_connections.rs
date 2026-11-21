//! 内联页签只切换投影；授权、策略、目录及解绑继续使用原请求 owner，不重建或取消在途请求。
use super::super::device_authorization_ui::{
    self as auth,
    settings::{self, Section},
};
use super::*;
#[path = "pin_settings_connection_fields.rs"]
mod projection_fields;
#[path = "pin_settings_targets.rs"]
pub(super) mod target_list;
#[cfg(test)]
#[path = "pin_settings_connection_state_tests.rs"]
mod tests;
#[derive(Default)]
pub(super) struct Session {
    pub receive: bool,
    snapshot: Option<settings::Snapshot>,
    fields: Vec<Vec<(Field, bool)>>,
    targets: Option<target_list::Popup>,
}
const LOOM: u16 = 201;
const SEND: u16 = 202;
const RECEIVE: u16 = 203;
const FLOW: u16 = 204;
const ONE_WAY: u16 = 205;
const TWO_WAY: u16 = 206;
const POLICY: u16 = 207;
const DEVICES: u16 = 208;
const GROUPS: u16 = 209;
const COPY: u16 = 210;
const UNLINK: u16 = 211;
pub(super) enum ReceiveSource {
    Lan(std::sync::Arc<crate::projection_delivery::journal::Ticket>),
    Account(crate::projection_account::scene::Binding),
}
impl ReceiveSource {
    fn id(&self) -> &str {
        match self {
            Self::Lan(ticket) => ticket.envelope().projection_id(),
            Self::Account(binding) => binding.envelope().projection_id(),
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Lan(a), Self::Lan(b)) => std::sync::Arc::ptr_eq(a, b),
            (Self::Account(a), Self::Account(b)) => a == b,
            _ => false,
        }
    }
}
pub(super) fn source(state: &State) -> Option<ReceiveSource> {
    let view = state.authorization.borrow();
    if let Some(binding) = view.account.bindings.iter().find(|binding| {
        !binding.is_source()
            && !binding.stopped()
            && binding.unit() == state.unit_id.as_str()
            && Some(binding.origin()) == view.origin.as_deref()
    }) {
        return Some(ReceiveSource::Account(binding.clone()));
    }
    view.projection
        .recovery
        .saved
        .iter()
        .find(|ticket| {
            ticket.unit() == state.unit_id.as_str()
                && Some(ticket.origin()) == view.origin.as_deref()
        })
        .cloned()
        .map(ReceiveSource::Lan)
}
fn unlink(state: &State) -> Option<usize> {
    let ReceiveSource::Lan(ticket) = source(state)? else {
        return None;
    };
    state
        .authorization
        .borrow()
        .projection
        .recovery
        .entries
        .iter()
        .position(|entry| std::sync::Arc::ptr_eq(entry, &ticket))
        .map(|index| auth::projection::recovery::BASE + index)
}
fn account_unlink_ready(state: &State) -> bool {
    let view = state.authorization.borrow();
    !view.closed && !view.busy && !view.account.busy && view.request.is_none()
}
fn item(items: &[crate::native_menu::Item], command: usize) -> Option<&crate::native_menu::Item> {
    items.iter().find_map(|i| {
        if i.command == command {
            Some(i)
        } else {
            i.children
                .as_ref()
                .and_then(|children| item(children, command))
        }
    })
}
pub(super) unsafe fn refresh(
    state: &State,
    session: &mut super::Session,
) -> Result<Vec<Vec<(Field, bool)>>> {
    let fields = projection_fields::fields(state, session.connections.receive)?;
    let c = &mut session.connections;
    if c.fields != fields || !c.snapshot.as_ref().is_some_and(|s| s.same_inputs(state)) {
        session.context = session.context.wrapping_add(1);
        c.snapshot = Some(settings::Snapshot::capture(state));
        c.fields = fields.clone();
    }
    Ok(fields)
}
fn current(state: &State, id: u16) -> bool {
    let session = state.pin_settings.borrow();
    session
        .connections
        .snapshot
        .as_ref()
        .is_some_and(|s| s.matches(state))
        && session
            .connections
            .fields
            .iter()
            .flatten()
            .any(|(f, enabled)| f.id == id && *enabled)
}
pub(super) unsafe fn execute(owner: HWND, state: &State, id: u16) -> Result<bool> {
    if !(201..=228).contains(&id) {
        return Ok(false);
    }
    if !current(state, id) {
        return Ok(true);
    }
    if id == FLOW {
        return Ok(true);
    }
    if matches!(id, ONE_WAY | TWO_WAY) {
        auth::editing::choose_mode(
            state,
            if id == ONE_WAY {
                crate::projection_edit::Mode::OneWay
            } else {
                crate::projection_edit::Mode::TwoWay
            },
        )?;
        return Ok(true);
    }
    if id == 227 || id == 228 {
        auth::execute(
            owner,
            state,
            if id == 227 {
                auth::sender::RETRY
            } else {
                auth::sender::CANCEL
            },
        )?;
        return Ok(true);
    }
    if matches!(id, 224..=226) {
        let command = match id {
            225 => auth::editing::KEEP_LOCAL,
            226 => auth::editing::USE_REMOTE,
            _ if state.projection_edit.borrow().is_some() => auth::editing::SUBMIT,
            _ => auth::editing::INITIALIZE,
        };
        auth::execute(owner, state, command)?;
        return Ok(true);
    }
    if matches!(id, SEND | RECEIVE) {
        state.pin_settings.borrow_mut().connections.receive = id == RECEIVE;
        return Ok(true);
    }
    if matches!(id, 222 | 223) {
        let receive = state.pin_settings.borrow().connections.receive;
        auth::share::show(owner, state, receive, id == 222)?;
        return Ok(true);
    }
    if id == LOOM {
        let command = if state.authorization.borrow().connected {
            auth::CANCEL
        } else {
            auth::SELECT
        };
        auth::execute(owner, state, command)?;
        return Ok(true);
    }
    if id == COPY {
        if let Some(ticket) = source(state) {
            crate::desktop::clipboard::copy_text_checked(owner, ticket.id(), || {
                current(state, id) && source(state).is_some_and(|now| now.same(&ticket))
            })?;
        }
        return Ok(true);
    }
    if id == UNLINK {
        if let Some(ReceiveSource::Account(binding)) = source(state) {
            if account_unlink_ready(state) {
                state.authorization.borrow_mut().request = Some(auth::Request::AccountProjection(
                    Box::new(auth::account::Request::Stop(binding)),
                ));
                if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
                    state.authorization.borrow_mut().request = None;
                    return Err(error);
                }
            }
            return Ok(true);
        }
    }
    if matches!(id, DEVICES | GROUPS | 218) {
        if state
            .pin_settings
            .borrow()
            .connections
            .targets
            .as_ref()
            .is_some_and(|p| p.applying)
        {
            return Ok(true);
        }
        let panel = state.pin_settings.borrow().panel.clone();
        if let Some(anchor) = panel.and_then(|p| p.anchor(id)) {
            let popup = target_list::Popup::open(owner, state, id, anchor)?;
            state.pin_settings.borrow_mut().connections.targets = Some(popup);
        }
        return Ok(true);
    }
    let section = match id {
        LOOM => Section::Loom,
        FLOW | ONE_WAY | TWO_WAY => Section::Flow,
        POLICY => Section::Policy,
        DEVICES => Section::Devices,
        GROUPS => Section::Groups,
        UNLINK => Section::Receive,
        _ => return Ok(true),
    };
    let command = match id {
        ONE_WAY => Some(auth::editing::ONE_WAY),
        TWO_WAY => Some(auth::editing::TWO_WAY),
        UNLINK => unlink(state),
        _ => None,
    };
    if let Some(command) = command {
        super::super::tool_panel::popup::preserve(state, || {
            settings::choose_with(owner, state, section, |_| Ok(Some(command)))
        })?;
    } else if id != UNLINK {
        let panel = state.pin_settings.borrow().panel.clone();
        if let Some(anchor) = panel.and_then(|p| p.anchor(id)) {
            super::super::tool_panel::popup::preserve(state, || {
                settings::show(owner, state, section, anchor)
            })?;
        }
    }
    Ok(true)
}
pub(super) unsafe fn poll_targets(owner: HWND, state: &State) {
    let popup = state.pin_settings.borrow_mut().connections.targets.take();
    if let Some(mut popup) = popup {
        match popup.poll(owner, state) {
            Ok(true) => state.pin_settings.borrow_mut().connections.targets = Some(popup),
            Ok(false) => {}
            Err(error) => {
                state.authorization.borrow_mut().sender.status = format!("目标菜单已关闭：{error}")
            }
        }
    }
}
pub(super) fn close_targets(state: &State) {
    let popup = {
        let mut session = state.pin_settings.borrow_mut();
        if session
            .connections
            .targets
            .as_ref()
            .is_some_and(|p| p.applying)
        {
            return;
        }
        session.connections.targets.take()
    };
    drop(popup);
}
pub(in crate::desktop) unsafe fn translate_targets(state: &State, message: &MSG) -> bool {
    let popup = state.pin_settings.borrow_mut().connections.targets.take();
    let Some(popup) = popup else {
        return false;
    };
    let consumed = popup.translate(message);
    state.pin_settings.borrow_mut().connections.targets = Some(popup);
    consumed
}
