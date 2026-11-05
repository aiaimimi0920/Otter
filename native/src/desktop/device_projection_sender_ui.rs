//! 文档菜单的单目标发送请求；只固定 Unit/目录快照，不编码、不签名、不访问磁盘或网络。
use super::*;
use crate::{
    projection_delivery::{
        sender::binding::Binding,
        targets::{Directory, MAX_TARGETS},
    },
    workspace_id::UnitId,
};
use std::sync::Arc;
#[path = "device_projection_groups_ui.rs"]
pub(in crate::desktop) mod groups;
pub(in crate::desktop) const BASE: usize = 0x1601;
pub(in crate::desktop) const CANCEL: usize = BASE + MAX_TARGETS;
pub(in crate::desktop) const RETRY: usize = CANCEL + 1;
pub(in crate::desktop) const STOP_BASE: usize = RETRY + 1;
pub(in crate::desktop) struct StopSelection {
    pub binding: Binding,
    pub source: crate::session_source::Source,
}
pub(in crate::desktop) struct Selection {
    pub unit: UnitId,
    pub directory: Arc<Directory>,
    pub index: usize,
}
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub ready: bool,
    pub busy: bool,
    pub cancelable: bool,
    pub can_stop: bool,
    pub stop_source: Option<crate::session_source::Source>,
    pub status: String,
    pub bindings: Vec<Binding>,
    pub retry: Option<UnitId>,
    pub group_unit: Option<UnitId>,
    pub group_results: Vec<String>,
    pub group_stopping: bool,
}
pub(in crate::desktop) fn handles(command: usize) -> bool {
    groups::handles(command)
        || (BASE..BASE + MAX_TARGETS).contains(&command)
        || command == CANCEL
        || command == RETRY
        || (STOP_BASE..STOP_BASE + crate::projection_delivery::sender::binding::MAX_PER_UNIT)
            .contains(&command)
}
pub(in crate::desktop) fn stops<'a>(
    view: &'a super::View,
    unit: &'a UnitId,
) -> impl Iterator<Item = &'a Binding> {
    view.sender
        .bindings
        .iter()
        .filter(move |binding| binding.unit() == unit.as_str())
}
pub(in crate::desktop) fn stop_enabled(view: &super::View, binding: &Binding) -> bool {
    view.sender.can_stop
        && !binding.stopped()
        && !view.closed
        && !view.busy
        && !view.projection.busy
        && !view.targets.busy
        && view.request.is_none()
        && view.sender.stop_source.is_some()
}
pub(in crate::desktop) fn bound(view: &super::View, unit: &UnitId, target: &str) -> bool {
    view.sender.bindings.iter().any(|binding| {
        binding.unit() == unit.as_str()
            && Some(binding.envelope().origin()) == view.origin.as_deref()
            && binding.target() == target
    })
}
pub(in crate::desktop) fn enabled(view: &super::View, unit: &UnitId, index: usize) -> bool {
    view.sender.ready
        && !view.sender.busy
        && !view.closed
        && !view.busy
        && !view.projection.busy
        && !view.targets.busy
        && view.request.is_none()
        && view.targets.directory.as_ref().is_some_and(|directory| {
            Some(directory.origin()) == view.origin.as_deref()
                && directory.targets().get(index).is_some_and(|target| {
                    target.route().available() && !bound(view, unit, target.id())
                })
        })
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if groups::handles(command) {
        return groups::execute(hwnd, state, command);
    }
    if let Some(index) = command
        .checked_sub(STOP_BASE)
        .filter(|index| *index < crate::projection_delivery::sender::binding::MAX_PER_UNIT)
    {
        let request = {
            let view = state.authorization.borrow();
            let binding = stops(&view, &state.unit_id)
                .nth(index)
                .filter(|binding| stop_enabled(&view, binding))
                .ok_or_else(|| Error::new(E_ABORT, "Saved sender unavailable"))?;
            Request::StopProjectionSend(Box::new(StopSelection {
                binding: binding.clone(),
                source: view.sender.stop_source.as_ref().unwrap().clone(),
            }))
        };
        return queue(hwnd, state, request);
    }
    if command == RETRY {
        if state.authorization.borrow().sender.retry.as_ref() != Some(&state.unit_id) {
            return Err(Error::new(E_ABORT, "No original sender request to retry"));
        }
        return queue(
            hwnd,
            state,
            Request::RetryProjectionSend(state.unit_id.clone()),
        );
    }
    if command == CANCEL {
        if !state.authorization.borrow().sender.cancelable {
            return Err(Error::new(E_ABORT, "No pending sender"));
        }
        return queue(hwnd, state, Request::CancelProjectionSend);
    }
    let request = {
        let view = state.authorization.borrow();
        let index = command
            .checked_sub(BASE)
            .filter(|index| *index < MAX_TARGETS)
            .ok_or_else(|| Error::new(E_ABORT, "Invalid projection target"))?;
        if !enabled(&view, &state.unit_id, index) {
            return Err(Error::new(E_ABORT, "Projection target unavailable"));
        }
        Request::SendProjection(Selection {
            unit: state.unit_id.clone(),
            directory: view.targets.directory.as_ref().unwrap().clone(),
            index,
        })
    };
    queue(hwnd, state, request)
}
