//! Project targets from the current authoritative catalog and bindings; the selector separately owns the displayed draft.
use super::*;
pub(super) fn entries(state: &State, kind: u16) -> Vec<Entry> {
    let view = state.authorization.borrow();
    let Some(directory) = &view.targets.directory else {
        return Vec::new();
    };
    if kind == DEVICES {
        directory
            .targets()
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let bound =
                    auth::sender::stops(&view, &state.unit_id)
                        .enumerate()
                        .find(|(_, b)| {
                            b.target() == Some(target.id())
                                && !b.stopped()
                                && b.envelope().origin() == directory.origin()
                        });
                Entry {
                    id: target.id().into(),
                    name: if target.name().is_empty() {
                        target.id()
                    } else {
                        target.name()
                    }
                    .into(),
                    checked: bound.is_some(),
                    enabled: bound.map_or(
                        target.route().available()
                            && !auth::sender::bound(&view, &state.unit_id, target.id()),
                        |(_, binding)| !binding.stopped(),
                    ),
                    command: bound.map_or(auth::sender::BASE + index, |(i, _)| {
                        auth::sender::STOP_BASE + i
                    }),
                }
            })
            .collect()
    } else if kind == GROUPS {
        drop(view);
        let devices = entries(state, DEVICES);
        let view = state.authorization.borrow();
        let directory = view.targets.directory.as_ref().unwrap();
        directory
            .groups()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(i, group)| {
                let checked = !group.target_ids().is_empty()
                    && group.target_ids().iter().all(|id| {
                        auth::sender::stops(&view, &state.unit_id).any(|b| {
                            b.target() == Some(id)
                                && !b.stopped()
                                && b.envelope().origin() == directory.origin()
                        })
                    });
                Entry {
                    id: group.id().into(),
                    name: group.name().into(),
                    checked,
                    enabled: devices.iter().any(|row| {
                        group.target_ids().contains(&row.id)
                            && row.enabled
                            && (checked || !row.checked)
                    }),
                    command: if checked {
                        auth::sender::groups::STOP_BASE + i
                    } else {
                        auth::sender::groups::BASE + i
                    },
                }
            })
            .collect()
    } else {
        drop(view);
        let devices = entries(state, DEVICES);
        let view = state.authorization.borrow();
        let directory = view.targets.directory.as_ref().unwrap();
        directory
            .users()
            .iter()
            .map(|user| Entry {
                id: user.id().into(),
                name: user.name().into(),
                checked: !user.target_ids().is_empty()
                    && user.target_ids().iter().all(|id| {
                        auth::sender::stops(&view, &state.unit_id).any(|b| {
                            b.target() == Some(id)
                                && !b.stopped()
                                && b.envelope().origin() == directory.origin()
                        })
                    }),
                enabled: devices
                    .iter()
                    .any(|row| user.target_ids().contains(&row.id) && row.enabled),
                command: 0,
            })
            .collect()
    }
}
