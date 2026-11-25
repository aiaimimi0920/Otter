//! Reuse painted Presentation layout order; cross-placement focus must not penetrate noninteractive upper layers.
use super::*;
use crate::tile_surface_focus::Focus;
pub(in crate::desktop::tile_window::render) fn font(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    focus: &Focus,
) -> Option<crate::surface_presentation::ControlFont> {
    let art = frame.arts.get(&focus.anchor.view.instance_id)?;
    cache
        .get(&focus.anchor.view.instance_id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?
        .presentation
        .control_font(&focus.node)
        .cloned()
}
pub(in crate::desktop::tile_window::render) fn at(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
) -> Option<Focus> {
    let (placement, _) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let p = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    crate::tile_surface_focus::at(frame, art, &p.presentation, pixel)
}
pub(in crate::desktop::tile_window::render) fn next(
    cache: &mut BTreeMap<String, Prepared>,
    frame: &Frame,
    current: Option<&Focus>,
    reverse: bool,
) -> Result<Option<Focus>> {
    let mut placements: Vec<_> = frame
        .layout
        .placements
        .iter()
        .filter(|p| p.interactive && p.source.kind == "surface")
        .collect();
    placements.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then(a.placement_id.cmp(&b.placement_id))
    });
    let mut candidates = Vec::new();
    for placement in placements {
        let Some(p) = cache.get(&placement.source.id).filter(|p| {
            frame
                .arts
                .get(&placement.source.id)
                .is_some_and(|a| Arc::ptr_eq(a, &p.owner))
        }) else {
            continue;
        };
        for (index, _) in p
            .presentation
            .geometry()
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, g)| p.presentation.can_focus(g.id()))
        {
            candidates.push((placement, index));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let index = current.and_then(|c| {
        candidates.iter().position(|(p, index)| {
            p.placement_id == c.anchor.placement
                && cache[&p.source.id].presentation.geometry().nodes()[*index].id() == c.node
        })
    });
    let start = match (index, reverse) {
        (Some(i), true) => (i + candidates.len() - 1) % candidates.len(),
        (Some(i), false) => (i + 1) % candidates.len(),
        (None, true) => candidates.len() - 1,
        (None, false) => 0,
    };
    let mut budget = 8192;
    for n in 0..candidates.len() {
        let index = if reverse {
            (start + candidates.len() - n) % candidates.len()
        } else {
            (start + n) % candidates.len()
        };
        let (placement, position) = candidates[index];
        let p = cache.get_mut(&placement.source.id).unwrap();
        // Restore the same scene/scroll snapshot per attempt, preserving original projection order; copy only the ID about to be probed.
        let node = p.presentation.geometry().nodes()[position].id().to_owned();
        let before = p.presentation.scroll_state();
        let result = (|| {
            let Some(viewport) = crate::tile_surface_geometry::viewport(frame, &p.owner, placement)
            else {
                return Ok(None);
            };
            if !p.presentation.reveal_focus_in(&node, viewport)? {
                return Ok(None);
            }
            crate::tile_surface_focus::find(
                frame,
                &p.owner,
                &p.presentation,
                placement,
                &node,
                &mut budget,
            )
            .map_err(|e| Error::new(E_INVALIDARG, e))
        })();
        // Planning never changes painted geometry or consumes IME/drafts; even search-budget failure rolls back.
        if p.presentation.scroll_state() != before {
            p.presentation.restore_scroll(&before)?;
        }
        scroll::reembed(p)?;
        if let Some(f) = result? {
            return Ok(Some(f));
        }
    }
    Ok(None)
}
