//! Generate immutable click events from actually painted Presentation; ordinary and Wall hosts share encoding without constructing mount Bindings.
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
            .ok_or("tile-art-hit-invalid")?;
    if placement.source.id != art.view.instance_id {
        return Err("tile-art-source-changed");
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
        // Collect at most once per bubbling batch; actions absent or not requesting forms do not traverse scenes.
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
            .map_err(|_| "tile-art-click-encode-failed")?;
        bytes += wire.len();
        if bytes > crate::surface_click_event::MAX_EVENT_BYTES {
            return Err("tile-art-click-batch-limit");
        }
        let event = serde_json::from_slice(&wire).map_err(|_| "tile-art-click-encode-failed")?;
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
