//! Tile滚动只重投影原Presentation；候选搜索回滚、不消耗草稿，提交后重装嵌入绘制缓存。
use super::*;
use crate::{surface_scroll::Scroll, tile_surface_focus::Focus, tile_surface_text::Field};
#[path = "tile_surface_scroll_memory.rs"]
mod memory;
pub(in crate::desktop::tile_window::render) use memory::Memory;
pub(in crate::desktop::tile_window::render) fn scrollbar(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
) -> Option<crate::tile_surface_scrollbar::Field> {
    let (placement, _) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let p = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    crate::tile_surface_scrollbar::Field::at(frame, art, &p.presentation, pixel)
}
pub(in crate::desktop::tile_window::render) fn current_bar(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    field: &crate::tile_surface_scrollbar::Field,
) -> Option<crate::tile_surface_scrollbar::Field> {
    let art = frame.arts.get(&field.instance)?;
    let p = cache
        .get(&field.instance)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    field.refreshed(frame, art, &p.presentation)
}
pub(in crate::desktop::tile_window::render) fn plan_bar(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    field: &crate::tile_surface_scrollbar::Field,
    value: f32,
) -> Option<Scroll> {
    current_bar(cache, frame, field)?;
    cache
        .get(&field.instance)?
        .presentation
        .planned_scrollbar(&field.bar.id, field.bar.axis, value)
}

pub(in crate::desktop::tile_window::render) fn retained_focus(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    field: &Focus,
) -> Option<Focus> {
    let art = frame.arts.get(&field.anchor.view.instance_id)?;
    let p = cache
        .get(&field.anchor.view.instance_id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    field.retained(frame, art, &p.presentation)
}
pub(in crate::desktop::tile_window::render) fn retained_text(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    field: &Field,
) -> Option<Field> {
    let art = frame.arts.get(&field.anchor.view.instance_id)?;
    let p = cache
        .get(&field.anchor.view.instance_id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    field.retained(frame, art, &p.presentation)
}
pub(super) fn reembed(p: &mut Prepared) -> Result<()> {
    p.presentation.embed_controls()?;
    for (id, value) in &p.text {
        p.presentation.preview_text(id, Some(value))?;
    }
    for (id, value) in &p.local {
        local::preview(&mut p.presentation, id, *value)?;
    }
    p.preview = None;
    Ok(())
}
pub(in crate::desktop::tile_window::render) fn wheel(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
    axis: usize,
    delta: f32,
    page: bool,
) -> Option<(String, Scroll)> {
    let (placement, uv) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let p = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    let point = [uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32];
    p.presentation
        .planned_wheel(point, axis, delta, page)
        .map(|s| (placement.source.id.clone(), s))
}
pub(in crate::desktop::tile_window::render) fn install(
    cache: &mut BTreeMap<String, Prepared>,
    frame: &Frame,
    id: &str,
    scroll: &Scroll,
) -> Result<bool> {
    let art = frame
        .arts
        .get(id)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Tile scroll source missing"))?;
    let p = cache
        .get_mut(id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))
        .ok_or_else(|| Error::new(E_INVALIDARG, "Tile scroll presentation changed"))?;
    if p.presentation.scroll_state() == *scroll {
        return Ok(false);
    }
    p.presentation.restore_scroll(scroll)?;
    reembed(p)?;
    Ok(true)
}
pub(in crate::desktop::tile_window::render) fn matches(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    id: &str,
    scroll: &Scroll,
) -> bool {
    cache.get(id).is_some_and(|p| {
        frame.arts.get(id).is_some_and(|a| Arc::ptr_eq(a, &p.owner))
            && p.presentation.scroll_state() == *scroll
    })
}
#[cfg(test)]
#[path = "tile_surface_scroll_render_tests.rs"]
mod tests;
