//! 从实际已绘制Presentation生成不可变点击事件；普通与Wall共用编码，不构造挂载Binding。
use crate::{
    surface_click_event::{Identity, Modifiers},
    surface_presentation::Presentation,
    surface_scene::{Kind, Node},
    tile_shared::Frame,
    tile_surface_actions::{Anchor, Command, Kind as CommandKind},
    tile_surface_owner::Art,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
pub(crate) fn prepare(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    pixel: [i32; 2],
    modifiers: Modifiers,
) -> Result<Vec<Command>, &'static str> {
    let (placement, uv) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)
            .ok_or("Art命中已失效")?;
    if placement.source.id != art.view.instance_id {
        return Err("Art来源已改变");
    }
    fn collect(node: &Node, values: &mut crate::surface_form_values::Values) {
        if !node.visible() {
            return;
        }
        if matches!(node.kind(), Kind::Input | Kind::Textarea | Kind::Number) {
            let v = node
                .props()
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            values.update(
                node.id(),
                if node.kind() == Kind::Number {
                    crate::surface_number::sanitized(v)
                } else {
                    v
                },
            );
        }
        for child in node.children() {
            collect(child, values);
        }
    }
    let point = [uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32];
    let mut route = p.click_route(point).peekable();
    if route.peek().is_none() {
        return Ok(Vec::new());
    }
    let anchor = Anchor {
        view: art.view.clone(),
        generation: art.generation,
        snapshot: art.snapshot.clone(),
        placement: placement.placement_id.clone(),
        pixel,
        scroll: p.scroll_state(),
    };
    let identity = Identity {
        instance: &art.view.instance_id,
        attachment: &art.view.attachment_id,
        generation: art.generation,
    };
    let mut result = Vec::new();
    let mut bytes = 0;
    let empty = serde_json::Map::new();
    let mut values = None;
    for action in route {
        // 一个冒泡批次最多采集一次；没有动作或未请求表单的动作不遍历场景。
        let fields = if action.includes_surface_values() {
            values
                .get_or_insert_with(|| {
                    let mut values = crate::surface_form_values::Values::default();
                    collect(p.scene().root(), &mut values);
                    values
                })
                .entries()
        } else {
            &empty
        };
        let (_, wire) = action
            .encode_identity(identity, art.snapshot.revision, modifiers, fields)
            .map_err(|_| "Art点击编码失败")?;
        bytes += wire.len();
        if bytes > crate::surface_click_event::MAX_EVENT_BYTES {
            return Err("Art点击批次过大");
        }
        let event = serde_json::from_slice(&wire).map_err(|_| "Art点击编码失败")?;
        result.push(Command {
            edit: None,
            anchor: anchor.clone(),
            kind: CommandKind::Event(event),
            until: Instant::now() + Duration::from_millis(1500),
        });
    }
    Ok(result)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_surface_click_tests.rs"]
mod tests;
