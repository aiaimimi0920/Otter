//! JS emit 的父侧复核；DOM 输出不授予动作，绑定/版本与随机事件身份只能由宿主填写。
use super::surface_javascript_wire::{self as wire, Emitted};
use crate::{
    surface_click_event::{self as click, ClickEvent},
    surface_live_binding::Binding,
    surface_scene::{Node, Scene},
};
use std::io;
pub(super) fn safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b":_-".contains(&c))
}
fn find<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    if node.id() == id {
        return Some(node);
    }
    node.children().iter().find_map(|child| find(child, id))
}
pub(super) fn encode(
    value: &Emitted,
    authority: &Scene,
    binding: &Binding,
    revision: u64,
) -> io::Result<ClickEvent> {
    if !safe(&value.node_id)
        || !safe(&value.event)
        || !safe(&value.action)
        || find(authority.root(), &value.node_id).and_then(|node| node.events().get(&value.event))
            != Some(&value.action)
    {
        return Err(wire::invalid());
    }
    let class = match value.class.as_str() {
        "discrete" => "discrete",
        "continuous" => "continuous",
        "commit" => "commit",
        _ => return Err(wire::invalid()),
    };
    if wire::encode(&value.payload)?.len() > 60 * 1024 {
        return Err(wire::invalid());
    }
    click::encode(
        binding,
        revision,
        click::Declaration {
            node_id: &value.node_id,
            event: &value.event,
            action: &value.action,
            class,
            modifiers: None,
            payload: &value.payload,
        },
    )
    .map(ClickEvent::javascript)
    .map_err(|_| wire::invalid())
}
