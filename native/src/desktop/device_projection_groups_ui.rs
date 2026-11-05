//! 设备组只固定目录身份和Unit；逐目标发送/持久化由Application的原单在途链拥有。
use super::*;
use crate::projection_delivery::targets::MAX_GROUPS;
pub(in crate::desktop) const BASE: usize = 0x1700;
pub(in crate::desktop) const STOP_BASE: usize = BASE + MAX_GROUPS;
pub(in crate::desktop) struct Selection {
    pub unit: UnitId,
    pub directory: Arc<Directory>,
    pub index: usize,
}
pub(in crate::desktop) struct StopSelection {
    pub group: Selection,
    pub source: crate::session_source::Source,
}
pub(super) fn handles(command: usize) -> bool {
    (BASE..STOP_BASE + MAX_GROUPS).contains(&command)
}
pub(in crate::desktop) fn stop_enabled(
    view: &super::super::View,
    unit: &UnitId,
    index: usize,
) -> bool {
    view.targets.directory.as_ref().is_some_and(|directory| {
        view.origin.as_deref() == Some(directory.origin())
            && directory
                .groups()
                .and_then(|groups| groups.get(index))
                .is_some_and(|group| {
                    super::stops(view, unit).any(|binding| {
                        binding.envelope().origin() == directory.origin()
                            && group.target_ids().iter().any(|id| id == binding.target())
                            && super::stop_enabled(view, binding)
                    })
                })
    })
}
pub(in crate::desktop) fn enabled(view: &super::super::View, unit: &UnitId, index: usize) -> bool {
    view.targets.directory.as_ref().is_some_and(|directory| {
        directory
            .groups()
            .and_then(|groups| groups.get(index))
            .is_some_and(|group| {
                directory.targets().iter().enumerate().any(|(i, target)| {
                    group.target_ids().iter().any(|id| id == target.id())
                        && super::enabled(view, unit, i)
                })
            })
    })
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    let request = {
        let view = state.authorization.borrow();
        let stopping = command >= STOP_BASE;
        let index = command
            .checked_sub(if stopping { STOP_BASE } else { BASE })
            .filter(|i| *i < MAX_GROUPS)
            .ok_or_else(|| Error::new(E_ABORT, "Invalid projection group"))?;
        if !(if stopping {
            stop_enabled(&view, &state.unit_id, index)
        } else {
            enabled(&view, &state.unit_id, index)
        }) {
            return Err(Error::new(E_ABORT, "Projection group unavailable"));
        }
        let group = Selection {
            unit: state.unit_id.clone(),
            directory: view.targets.directory.as_ref().unwrap().clone(),
            index,
        };
        if stopping {
            Request::StopProjectionGroup(StopSelection {
                group,
                source: view.sender.stop_source.as_ref().unwrap().clone(),
            })
        } else {
            Request::SendProjectionGroup(group)
        }
    };
    queue(hwnd, state, request)
}
