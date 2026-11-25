//! The original mainline compact inline layout; project only verifiable state, never fabricate missing Loom identity, group counts, or sharing capabilities.
use super::*;
// Translated captions and display-only policy values do not retire geometry drafts.
// Command identity, structure, active state, eligibility, and owner snapshots still do.
pub(super) fn same_commands(a: &[Vec<(Field, bool)>], b: &[Vec<(Field, bool)>]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|((a, a_enabled), (b, b_enabled))| {
                    a.id == b.id
                        && a_enabled == b_enabled
                        && match (&a.kind, &b.kind) {
                            (Kind::Action { active: a, .. }, Kind::Action { active: b, .. }) => {
                                a == b
                            }
                            (Kind::Label { .. }, Kind::Label { .. })
                            | (Kind::Dropdown { .. }, Kind::Dropdown { .. }) => true,
                            _ => a.kind == b.kind,
                        }
                })
        })
}
fn icon(name: &str) -> neuro_mini_ui::Icon {
    neuro_mini_ui::Icon::svg(match name {
        "Loom" => include_str!("../../assets/edit-strip/SettingsLoom.svg"),
        "Send" => include_str!("../../assets/edit-strip/SettingsSend.svg"),
        "Receive" => include_str!("../../assets/edit-strip/SettingsReceive.svg"),
        "Flow" => include_str!("../../assets/edit-strip/SettingsFlow.svg"),
        "OneWay" => include_str!("../../assets/edit-strip/SettingsOneWay.svg"),
        "OneWayReceive" => include_str!("../../assets/edit-strip/SettingsOneWayReceive.svg"),
        "TwoWay" => include_str!("../../assets/edit-strip/SettingsTwoWay.svg"),
        "Export" => include_str!("../../assets/edit-strip/SettingsExport.svg"),
        "Import" => include_str!("../../assets/edit-strip/SettingsImport.svg"),
        "Qr" => include_str!("../../assets/edit-strip/SettingsQr.svg"),
        "Link" => include_str!("../../assets/edit-strip/SettingsLink.svg"),
        "Devices" => include_str!("../../assets/edit-strip/SettingsDevices.svg"),
        "Groups" => include_str!("../../assets/edit-strip/SettingsGroups.svg"),
        "Friends" => include_str!("../../assets/edit-strip/SettingsFriends.svg"),
        "Copy" => include_str!("../../assets/edit-strip/SettingsCopy.svg"),
        "Unlink" => include_str!("../../assets/edit-strip/SettingsUnlink.svg"),
        "Source" => include_str!("../../assets/edit-strip/SettingsSource.svg"),
        _ => unreachable!("Unknown sidebar icon"),
    })
}
fn field(id: u16, label: &'static str, name: &str, kind: Kind, enabled: bool) -> (Field, bool) {
    (
        Field {
            id,
            label: crate::ui_text::text(label),
            icon: (!name.is_empty()).then(|| icon(name)),
            kind,
        },
        enabled,
    )
}
fn action(
    id: u16,
    label: &'static str,
    name: &str,
    value: &str,
    width: u16,
    active: bool,
    enabled: bool,
) -> (Field, bool) {
    field(
        id,
        label,
        name,
        Kind::Action {
            value: value.into(),
            width,
            active,
        },
        enabled,
    )
}
fn label(id: u16, label: &'static str, name: &str, value: &str, width: u16) -> (Field, bool) {
    field(
        id,
        label,
        name,
        Kind::Label {
            value: value.into(),
            width,
        },
        true,
    )
}
pub(super) unsafe fn fields(state: &State, receive: bool) -> Result<Vec<Vec<(Field, bool)>>> {
    let flow = settings::items(Section::Flow, state)?;
    let available = |id| item(&flow, id).is_some_and(|i| i.enabled);
    let mode = auth::editing::mode(state);
    let mut projection = vec![
        action(SEND, "pin-send", "Send", "", 58, !receive, true),
        action(RECEIVE, "pin-receive", "Receive", "", 58, receive, true),
        label(FLOW, "pin-flow", "Flow", "", 18),
        action(
            ONE_WAY,
            "pin-one-way",
            if receive { "OneWayReceive" } else { "OneWay" },
            "",
            48,
            mode == crate::projection_edit::Mode::OneWay,
            !receive && auth::editing::can_choose_mode(state),
        ),
        action(
            TWO_WAY,
            "pin-two-way",
            "TwoWay",
            "",
            49,
            mode == crate::projection_edit::Mode::TwoWay,
            !receive && auth::editing::can_choose_mode(state),
        ),
    ];
    let conflict = state
        .projection_edit
        .borrow()
        .as_ref()
        .is_some_and(|o| o.authoritative().conflict().is_some());
    if conflict {
        projection.push(action(
            225,
            "pin-keep-local",
            "",
            crate::ui_text::text("pin-keep-local"),
            96,
            false,
            available(auth::editing::KEEP_LOCAL),
        ));
        projection.push(action(
            226,
            "pin-use-remote",
            "",
            crate::ui_text::text("pin-use-remote"),
            96,
            false,
            available(auth::editing::USE_REMOTE),
        ));
    } else if state.authorization.borrow().editing.recovery {
        let command = if state.projection_edit.borrow().is_some() {
            auth::editing::SUBMIT
        } else {
            auth::editing::INITIALIZE
        };
        projection.push(action(
            224,
            "common-retry",
            "",
            crate::ui_text::text("common-retry"),
            116,
            false,
            available(command),
        ));
    }
    if receive {
        let policy = settings::items(Section::Policy, state)?;
        let view = state.authorization.borrow();
        let value = if view.consent.paused() {
            crate::ui_text::text("pin-paused")
        } else {
            match view.consent.policy() {
                crate::projection_receive_policy::Policy::Auto => {
                    crate::ui_text::text("pin-policy-auto")
                }
                crate::projection_receive_policy::Policy::Confirm => {
                    crate::ui_text::text("pin-policy-confirm")
                }
                crate::projection_receive_policy::Policy::Reject => {
                    crate::ui_text::text("pin-policy-reject")
                }
            }
        };
        projection.extend([
            label(
                220,
                "pin-receive-policy",
                "",
                crate::ui_text::text("pin-receive-policy-short"),
                48,
            ),
            field(
                POLICY,
                "pin-local-receive-policy",
                "",
                Kind::Dropdown {
                    value: value.into(),
                    width: 68,
                },
                item(&policy, auth::policy::AUTO).is_some_and(|i| i.enabled),
            ),
        ]);
    }
    projection.extend([
        label(
            221,
            if receive { "pin-import" } else { "pin-export" },
            if receive { "Import" } else { "Export" },
            "",
            18,
        ),
        action(
            222,
            "pin-qr-code",
            "Qr",
            "",
            48,
            false,
            auth::share::available(state, receive),
        ),
        action(
            223,
            "pin-link",
            "Link",
            "",
            49,
            false,
            auth::share::available(state, receive),
        ),
    ]);
    if receive {
        let ticket = source(state);
        let id = ticket.as_ref().map_or("-", ReceiveSource::id);
        let menu = settings::items(Section::Receive, state)?;
        projection.extend([
            label(219, "pin-source-id", "Source", id, 61),
            action(
                COPY,
                "pin-copy-source-id",
                "Copy",
                "",
                27,
                false,
                ticket.is_some(),
            ),
            action(
                UNLINK,
                "pin-leave-projection",
                "Unlink",
                "",
                27,
                false,
                match ticket.as_ref() {
                    Some(ReceiveSource::Account(_)) => account_unlink_ready(state),
                    _ => unlink(state).is_some_and(|id| item(&menu, id).is_some_and(|i| i.enabled)),
                },
            ),
        ]);
    } else {
        let view = state.authorization.borrow();
        let (count, pending) = auth::sender::stops(&view, &state.unit_id)
            .filter(|b| !b.stopped() && Some(b.envelope().origin()) == view.origin.as_deref())
            .fold((0, 0), |(count, pending), b| {
                (count + 1, pending + usize::from(!b.linked()))
            });
        projection.extend([
            label(219, "pin-projection-targets", "Send", "", 18),
            action(
                DEVICES,
                "pin-devices",
                "Devices",
                &count.to_string(),
                32,
                false,
                true,
            ),
            action(
                GROUPS,
                "pin-device-groups",
                "Groups",
                &view
                    .targets
                    .directory
                    .as_ref()
                    .and_then(|d| d.groups())
                    .map_or(0, |g| g.len())
                    .to_string(),
                32,
                false,
                true,
            ),
            action(
                218,
                "pin-users",
                "Friends",
                &view
                    .targets
                    .directory
                    .as_ref()
                    .map_or(0, |d| d.users().len())
                    .to_string(),
                32,
                false,
                true,
            ),
        ]);
        if count > 0 {
            let status = if pending == 0 {
                crate::ui_text::format("pin-projection-received", &[("count", count.into())])
            } else {
                crate::ui_text::format(
                    "pin-projection-pending",
                    &[
                        ("count", (count - pending).into()),
                        ("pending", pending.into()),
                    ],
                )
            };
            projection.push(label(229, "pin-projection-status", "", &status, 116));
        }
    }
    let view = state.authorization.borrow();
    if view.sender.retry.as_ref() == Some(&state.unit_id) {
        projection.push(action(
            227,
            "pin-retry-projection",
            "",
            crate::ui_text::text("common-retry"),
            58,
            false,
            view.request.is_none(),
        ));
        projection.push(action(
            228,
            "pin-cancel-projection",
            "",
            crate::ui_text::text("common-cancel"),
            58,
            false,
            view.sender.cancelable && view.request.is_none(),
        ));
    }
    let name = if view.connected {
        view.loom_name.as_str()
    } else {
        crate::ui_text::text("pin-disconnected")
    };
    Ok(vec![
        vec![
            label(217, "pin-loom", "Loom", name, 82),
            action(
                LOOM,
                if view.connected {
                    "pin-disconnect"
                } else {
                    "pin-connect"
                },
                if view.connected { "Unlink" } else { "Link" },
                "",
                34,
                false,
                view.request.is_none(),
            ),
        ],
        projection,
    ])
}
