//! 本地值只投影进原Presentation缓存；同一paint身份/合同/源值不匹配时不显示旧草稿。
use super::*;
use crate::tile_surface_local::{Entry, Value};
pub(in crate::desktop::tile_window) fn apply(
    cache: &mut BTreeMap<String, Prepared>,
    frame: Option<&Frame>,
    rows: &[Entry],
) -> Result<()> {
    for p in cache.values_mut() {
        let desired: BTreeMap<_, _> = rows
            .iter()
            .filter(|r| {
                frame
                    .and_then(|f| r.focus.retained(f, &p.owner, &p.presentation))
                    .is_some_and(|f| r.focus.same_local(&f))
                    && p.presentation
                        .scene()
                        .root()
                        .visible_node(&r.focus.node)
                        .and_then(crate::tile_surface_local::source_node)
                        == Some(r.source)
                    && p.presentation
                        .scene()
                        .root()
                        .visible_node(&r.focus.node)
                        .map(crate::tile_surface_local::stamp)
                        == Some(r.stamp)
            })
            .map(|r| (r.focus.node.clone(), r.value))
            .collect();
        for (id, value) in &p.local {
            if !desired.contains_key(id) {
                match value {
                    Value::Select(_) => {
                        p.presentation.preview_text(id, None)?;
                    }
                    Value::Slider(_) => {
                        p.presentation.preview_slider(id, None);
                    }
                    Value::Switch(_) => {}
                }
            }
        }
        for (id, value) in &desired {
            if p.local.get(id) == Some(value) {
                continue;
            }
            preview(&mut p.presentation, id, *value)?;
        }
        p.local = desired;
    }
    Ok(())
}
pub(super) fn preview(p: &mut Presentation, id: &str, value: Value) -> Result<()> {
    match value {
        Value::Select(index) => {
            let node = p
                .scene()
                .root()
                .visible_node(id)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Tile select preview node"))?;
            let options = crate::surface_select::Options::parse(node)
                .map_err(|_| Error::new(E_INVALIDARG, "Tile select preview options"))?;
            let label = match index {
                Some(i) => options
                    .items()
                    .get(i)
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Tile select preview index"))?
                    .label(),
                None => "",
            };
            p.preview_text(id, Some(label))?;
        }
        Value::Slider(v) => {
            p.preview_slider(id, Some(v));
        }
        Value::Switch(_) => {}
    }
    Ok(())
}
