//! 已呈现的离散值动作；只读声明值，不创建文本草稿或伪造普通Surface挂载。
use crate::{
    surface_input_event::{Phase, Value},
    surface_presentation::Presentation,
    surface_scene::{Kind, Node, Scene},
    tile_shared::Frame,
    tile_surface_actions::{Anchor, Command, Kind as CommandKind},
    tile_surface_owner::Art,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
pub(crate) enum Control {
    Switch(bool),
    Select(crate::surface_select::Options),
}
pub(crate) struct Field {
    pub anchor: Anchor,
    pub node: String,
    pub control: Control,
    pub selection: Option<usize>,
    prefix: Vec<Command>,
}
pub(crate) fn node<'a>(root: &'a Node, id: &str) -> Option<&'a Node> {
    root.visible_node(id)
}
pub(crate) fn hit(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    pixel: [i32; 2],
    modifiers: crate::surface_click_event::Modifiers,
) -> Option<Field> {
    let (placement, uv) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    if placement.source.id != art.view.instance_id {
        return None;
    }
    let point = [uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32];
    let current = node(p.scene().root(), p.hit_test(point)?)?;
    if !current.visible() || current.props().get("disabled") == Some(&serde_json::Value::Bool(true))
    {
        return None;
    }
    let control = match current.kind() {
        Kind::Switch => Control::Switch(crate::surface_switch::checked(current)),
        Kind::Select => Control::Select(crate::surface_select::Options::parse(current).ok()?),
        _ => return None,
    };
    // 同普通Switch，祖先click属于激活边沿，先于change；Select子控件不合成click。
    let prefix = if current.kind() == Kind::Switch {
        crate::tile_surface_click::prepare(frame, art, p, pixel, modifiers).ok()?
    } else {
        Vec::new()
    };
    let selection = match &control {
        Control::Select(o) => o.source_index(),
        _ => None,
    };
    Some(Field {
        anchor: Anchor {
            view: art.view.clone(),
            generation: art.generation,
            snapshot: art.snapshot.clone(),
            placement: placement.placement_id.clone(),
            pixel,
            scroll: p.scroll_state(),
        },
        node: current.id().into(),
        control,
        selection,
        prefix,
    })
}
impl Field {
    pub fn same(&self, next: &Self) -> bool {
        self.anchor.view == next.anchor.view
            && self.anchor.generation == next.anchor.generation
            && Arc::ptr_eq(&self.anchor.snapshot, &next.anchor.snapshot)
            && self.anchor.placement == next.anchor.placement
            && self.node == next.node
    }
    pub fn commands(self, selected: Option<usize>) -> Result<Vec<Command>, &'static str> {
        let prior = self.selection;
        self.commands_after(selected, prior)
    }
    pub fn commands_after(
        mut self,
        selected: Option<usize>,
        prior: Option<usize>,
    ) -> Result<Vec<Command>, &'static str> {
        let scene = Scene::parse(&self.anchor.snapshot.scene).map_err(|_| "Art字段场景失效")?;
        let current = node(scene.root(), &self.node).ok_or("Art字段已移除")?;
        let value = match &self.control {
            Control::Switch(checked) => Value::Boolean(!checked),
            Control::Select(options) => {
                let Some(index) = selected.filter(|i| Some(*i) != prior) else {
                    return Ok(Vec::new());
                };
                Value::Text(options.value_at(index).ok_or("Art选项已失效")?)
            }
        };
        let identity = crate::surface_click_event::Identity {
            instance: &self.anchor.view.instance_id,
            attachment: &self.anchor.view.attachment_id,
            generation: self.anchor.generation,
        };
        let encoded = crate::surface_input_event::encode_identity(
            current,
            identity,
            self.anchor.snapshot.revision,
            Phase::Change,
            value,
        )
        .map_err(|_| "Art字段编码失败")?;
        let Some((_, bytes)) = encoded else {
            // 无change仍保Switch激活边沿的祖先click，不伪造值动作。
            return Ok(self.prefix);
        };
        let event = serde_json::from_slice(&bytes).map_err(|_| "Art字段编码失败")?;
        let total = bytes.len()
            + self
                .prefix
                .iter()
                .map(|c| match &c.kind {
                    CommandKind::Event(v) => v.to_string().len(),
                    _ => 0,
                })
                .sum::<usize>();
        if total > crate::surface_click_event::MAX_EVENT_BYTES {
            return Err("Art字段批次过大");
        }
        let until = Instant::now() + Duration::from_millis(1500);
        for c in &mut self.prefix {
            c.until = until;
        }
        self.prefix.push(Command {
            edit: None,
            anchor: self.anchor,
            kind: CommandKind::Event(event),
            until,
        });
        Ok(self.prefix)
    }
}
/// 新鲜Presentation重新核真实节点/选项/动作；不能把任意JSON改投change。
pub(crate) fn validates(p: &Presentation, point: [f32; 2], event: &serde_json::Value) -> bool {
    let Some(id) = event["nodeId"]
        .as_str()
        .filter(|id| p.hit_test(point) == Some(id))
    else {
        return false;
    };
    let Some(current) = node(p.scene().root(), id) else {
        return false;
    };
    if !current.visible()
        || current.props().get("disabled") == Some(&serde_json::Value::Bool(true))
        || !current.events().contains_key("change")
        || current.events().get("change").map(String::as_str) != event["action"].as_str()
        || event["event"] != "change"
        || event["class"] != "commit"
        || event.get("modifiers").is_some()
        || event["payload"].as_object().is_none_or(|p| p.len() != 1)
    {
        return false;
    }
    let value = &event["payload"]["value"];
    match current.kind() {
        Kind::Switch => value.as_bool() == Some(!crate::surface_switch::checked(current)),
        Kind::Select => crate::surface_select::Options::parse(current).is_ok_and(|o| {
            value
                .as_str()
                .is_some_and(|v| o.items().iter().any(|i| i.value() == v))
        }),
        _ => false,
    }
}
#[cfg(test)]
#[path = "tile_surface_value_tests.rs"]
pub(crate) mod tests;
