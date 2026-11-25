//! 原主干的紧凑内联布局；只投影可验证状态，缺失的 Loom 身份、组计数和分享能力不伪造。
use super::*;
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
            label,
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
        action(SEND, "投射", "Send", "", 58, !receive, true),
        action(RECEIVE, "被投射", "Receive", "", 58, receive, true),
        label(FLOW, "数据流向", "Flow", "", 18),
        action(
            ONE_WAY,
            "单向数据",
            if receive { "OneWayReceive" } else { "OneWay" },
            "",
            48,
            mode == crate::projection_edit::Mode::OneWay,
            !receive && auth::editing::can_choose_mode(state),
        ),
        action(
            TWO_WAY,
            "双向数据",
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
            "保留本地",
            "",
            "保留本地",
            58,
            false,
            available(auth::editing::KEEP_LOCAL),
        ));
        projection.push(action(
            226,
            "采用远端",
            "",
            "采用远端",
            58,
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
            "重试",
            "",
            "重试",
            116,
            false,
            available(command),
        ));
    }
    if receive {
        let policy = settings::items(Section::Policy, state)?;
        let view = state.authorization.borrow();
        let value = if view.consent.paused() {
            "暂停"
        } else {
            match view.consent.policy() {
                crate::projection_receive_policy::Policy::Auto => "自动",
                crate::projection_receive_policy::Policy::Confirm => "接收",
                crate::projection_receive_policy::Policy::Reject => "拒绝",
            }
        };
        projection.extend([
            label(220, "接收策略", "", "接收策略", 48),
            field(
                POLICY,
                "本机接收策略",
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
            if receive { "导入" } else { "导出" },
            if receive { "Import" } else { "Export" },
            "",
            18,
        ),
        action(
            222,
            "二维码",
            "Qr",
            "",
            48,
            false,
            auth::share::available(state, receive),
        ),
        action(
            223,
            "链接",
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
            label(219, "来源 ID", "Source", id, 61),
            action(COPY, "复制来源 ID", "Copy", "", 27, false, ticket.is_some()),
            action(
                UNLINK,
                "退出投射（本地图片保留）",
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
            label(219, "投射目标", "Send", "", 18),
            action(
                DEVICES,
                "设备",
                "Devices",
                &count.to_string(),
                32,
                false,
                true,
            ),
            action(
                GROUPS,
                "设备组",
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
                "用户",
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
                format!("已接收 {count}")
            } else {
                format!("已接收 {} · 等待 {pending}", count - pending)
            };
            projection.push(label(229, "投射状态", "", &status, 116));
        }
    }
    let view = state.authorization.borrow();
    if view.sender.retry.as_ref() == Some(&state.unit_id) {
        projection.push(action(
            227,
            "重试投射",
            "",
            "重试",
            58,
            false,
            view.request.is_none(),
        ));
        projection.push(action(
            228,
            "取消投射",
            "",
            "取消",
            58,
            false,
            view.sender.cancelable && view.request.is_none(),
        ));
    }
    let name = if view.connected {
        view.loom_name.as_str()
    } else {
        "未连接"
    };
    Ok(vec![
        vec![
            label(217, "Loom", "Loom", name, 82),
            action(
                LOOM,
                "Loom 连接",
                "",
                if view.connected { "断开" } else { "连接" },
                34,
                false,
                view.request.is_none(),
            ),
        ],
        projection,
    ])
}
