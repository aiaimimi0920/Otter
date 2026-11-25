//! Project only mainline node names, original SVGs, and existing node actions; own no pixels, executors, or parameter copies.
use super::*;
pub(super) fn capture(state: &State) -> Model {
    let title = state
        .selected_art
        .borrow()
        .as_ref()
        .and_then(|selected| {
            selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(crate::loom_capability::Capability::new)
                .find(|c| c.id == selected.parameters.art_id())
                .map(|c| c.label.to_owned())
        })
        .unwrap_or_else(|| "贴图".into());
    let mut checked = [false; 5];
    for field in desktop::node_settings::fields(state) {
        checked[match field.action {
            Action::Endpoints => 0,
            Action::Upstream => 1,
            Action::Parameter => 2,
            Action::Notify => 3,
            Action::Relay => 4,
        }] = field.checked;
    }
    Model {
        stamp: Stamp::capture(state),
        settings: state.node_settings.get(),
        title,
        art: state.is_art(),
        has_execution: state.selected_art.borrow().is_some(),
        enabled: parameter_panel::available(state),
        run: desktop::execution_start::available(state),
        checked,
    }
}
pub(super) fn action(id: u16) -> Option<Action> {
    Some(match id {
        2 => Action::Endpoints,
        3 => Action::Upstream,
        4 => Action::Parameter,
        5 => Action::Notify,
        6 => Action::Relay,
        _ => return None,
    })
}
pub(super) fn intent(id: u16, model: &Model) -> Option<Option<Action>> {
    if !available(model, id) {
        return None;
    }
    Some(action(id))
}
fn available(model: &Model, id: u16) -> bool {
    model.enabled
        && match id {
            2 => true,
            3 | 4 => model.art && model.has_execution,
            5 => !model.art || model.has_execution,
            6 => !model.art,
            RUN => model.art && model.run,
            _ => false,
        }
}
pub(super) fn buttons(model: &Model) -> Vec<DrawerButton> {
    let icon = |source| Icon::svg(source);
    let mut items = vec![DrawerButton {
        id: 2,
        label: if model.settings.hide_collapsed_endpoints {
            "收起时隐藏端点；点击改为显示"
        } else {
            "收起时显示端点；点击改为隐藏"
        }
        .into(),
        icon: icon(if model.settings.hide_collapsed_endpoints {
            include_str!("../../assets/edit-strip/NodeEndpointsHidden.svg")
        } else {
            include_str!("../../assets/edit-strip/NodeEndpoints.svg")
        }),
        text: None,
        active: !model.settings.hide_collapsed_endpoints,
        enabled: available(model, 2),
    }];
    if model.art {
        items.extend([
            DrawerButton {
                id: 3,
                label: "上游变化自动执行".into(),
                icon: icon(include_str!("../../assets/edit-strip/NodeUpstream.svg")),
                text: None,
                active: model.checked[1],
                enabled: available(model, 3),
            },
            DrawerButton {
                id: 4,
                label: "参数变化自动执行".into(),
                icon: icon(include_str!("../../assets/edit-strip/NodeParameter.svg")),
                text: None,
                active: model.checked[2],
                enabled: available(model, 4),
            },
        ]);
    }
    items.push(DrawerButton {
        id: 5,
        label: "通知下游：允许本节点更新触发下游执行".into(),
        icon: icon(include_str!("../../assets/edit-strip/NodeNotify.svg")),
        text: None,
        active: model.checked[3],
        enabled: available(model, 5),
    });
    if !model.art {
        items.push(DrawerButton {
            id: 6,
            label: "接受上级贴图编辑传导".into(),
            icon: icon(include_str!("../../assets/edit-strip/NodeRelay.svg")),
            text: None,
            active: model.checked[4],
            enabled: available(model, 6),
        });
    }
    if model.art {
        items.push(DrawerButton {
            id: RUN,
            label: "执行一次".into(),
            icon: icon(include_str!("../../assets/edit-strip/NodeRun.svg")),
            text: Some("执行".into()),
            active: false,
            enabled: available(model, RUN),
        });
    }
    items
}
