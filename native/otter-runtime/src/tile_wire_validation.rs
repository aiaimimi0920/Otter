//! 除单字段外核快照内引用、唯一归属和几何；拒绝坏状态，不降格成成功空墙。
use super::*;
use std::collections::HashSet;
fn require(v: bool) -> Result<()> {
    v.then_some(()).ok_or("tile_state_invalid")
}
fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.into_iter().all(|v| seen.insert(v))
}
fn rect(r: &Rect) -> bool {
    [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite())
        && r.x >= -1e6
        && r.y >= -1e6
        && r.x <= 1e6
        && r.y <= 1e6
        && r.width >= 1. / 65536.
        && r.height >= 1. / 65536.
        && r.width <= 1e6
        && r.height <= 1e6
        && r.x + r.width <= 1e6
        && r.y + r.height <= 1e6
        && r.x + r.width > r.x
        && r.y + r.height > r.y
}
fn choices(values: &[String], allowed: &[&str]) -> bool {
    values.len() <= allowed.len()
        && unique(values.iter().map(String::as_str))
        && values.iter().all(|v| allowed.contains(&v.as_str()))
}
fn endpoint(e: &Endpoint) -> bool {
    e.protocol_version == PROTOCOL
        && [&e.endpoint_id, &e.device_id, &e.output_id]
            .iter()
            .all(|s| identifier(s))
        && (1..=16384).contains(&e.pixel_size.width)
        && (1..=16384).contains(&e.pixel_size.height)
        && !e.render_modes.is_empty()
        && choices(
            &e.render_modes,
            &["raw_bgra", "h264", "image", "surface_v1"],
        )
        && choices(
            &e.input_capabilities,
            &["pointer", "wheel", "keyboard", "text", "touch", "pen"],
        )
        && e.display.as_ref().is_none_or(|d| {
            !d.name.trim().is_empty()
                && d.name.chars().count() <= 256
                && !d.name.chars().any(char::is_control)
        })
}
pub(super) fn state(s: &State) -> Result<()> {
    require(
        s.protocol_version == PROTOCOL
            && s.revision <= MAX_SAFE
            && s.layouts.len() <= 64
            && s.endpoints.len() <= 256
            && s.presentations.len() <= 64,
    )?;
    require(s.revision > 0 || (s.endpoints.is_empty() && s.layouts.is_empty()))?;
    require(
        unique(s.layouts.iter().map(|l| l.wall_id.as_str()))
            && unique(s.endpoints.iter().map(|e| e.endpoint.endpoint_id.as_str())),
    )?;
    require(unique(
        s.layouts
            .iter()
            .flat_map(|l| l.tiles.iter().map(|t| t.endpoint_id.as_str())),
    ))?;
    let mut outputs = HashSet::new();
    for l in &s.layouts {
        require(
            l.protocol_version == PROTOCOL
                && identifier(&l.wall_id)
                && l.revision > 0
                && l.revision <= s.revision
                && rect(&l.bounds)
                && l.tiles.len() <= 64
                && l.placements.len() <= 256
                && unique(l.tiles.iter().map(|t| t.tile_id.as_str()))
                && unique(l.placements.iter().map(|p| p.placement_id.as_str())),
        )?;
        for (i, t) in l.tiles.iter().enumerate() {
            require(
                identifier(&t.tile_id)
                    && identifier(&t.endpoint_id)
                    && rect(&t.rect)
                    && l.bounds.contains(&t.rect)
                    && !l.tiles[..i].iter().any(|p| p.rect.intersects(&t.rect)),
            )?;
        }
        for p in &l.placements {
            require(
                identifier(&p.placement_id)
                    && identifier(&p.source.id)
                    && ["image", "live", "surface"].contains(&p.source.kind.as_str())
                    && (p.source.kind != "image" || (image_id(&p.source.id) && !p.interactive))
                    && rect(&p.rect)
                    && rect(&p.source_crop)
                    && (Rect {
                        x: 0.,
                        y: 0.,
                        width: 1.,
                        height: 1.,
                    })
                    .contains(&p.source_crop),
            )?;
        }
    }
    require(unique(s.presentations.iter().map(|p| p.wall_id.as_str())))?;
    for p in &s.presentations {
        require(
            s.layouts.iter().any(|l| l.wall_id == p.wall_id)
                && p.revision > 0
                && p.revision <= s.revision,
        )?;
    }
    if let Some(t) = &s.timing {
        require(
            identifier(&t.clock_id)
                && t.server_time_ms > 0
                && t.server_time_ms <= MAX_SAFE
                && t.scenes.len() == s.layouts.len()
                && unique(t.scenes.iter().map(|scene| scene.wall_id.as_str())),
        )?;
        for scene in &t.scenes {
            require(
                s.layouts
                    .iter()
                    .any(|l| l.wall_id == scene.wall_id && l.revision == scene.revision)
                    && scene.prepared_at_ms > 0
                    && scene.prepared_at_ms <= t.server_time_ms
                    && scene.activate_at_ms >= scene.prepared_at_ms
                    && scene.activate_at_ms <= scene.prepared_at_ms.saturating_add(10_000),
            )?;
        }
    }
    for e in &s.endpoints {
        require(
            endpoint(&e.endpoint) && outputs.insert((&e.endpoint.device_id, &e.endpoint.output_id)),
        )?;
        let layout = s.layouts.iter().find(|l| {
            l.tiles
                .iter()
                .any(|t| t.endpoint_id == e.endpoint.endpoint_id)
        });
        require(
            e.applied_revision
                .is_none_or(|v| e.online && layout.is_some_and(|l| l.revision == v)),
        )?;
        let presentation = s
            .presentations
            .iter()
            .find(|p| Some(p.wall_id.as_str()) == layout.map(|l| l.wall_id.as_str()));
        if let Some(r) = &e.presentation {
            let p = presentation.ok_or("tile_state_invalid")?;
            require(
                e.online
                    && r.revision == p.revision
                    && match p.mode {
                        Mode::Black => r.outcome == "applied" && e.applied_revision.is_none(),
                        Mode::Frozen => {
                            (r.outcome == "applied" && e.applied_revision.is_some())
                                || (r.outcome == "frame_unavailable"
                                    && e.applied_revision.is_none())
                        }
                    },
            )?;
        }
        if let Some(i) = &e.identification {
            require(
                e.online
                    && presentation.is_none()
                    && e.endpoint.display.as_ref().is_some_and(|d| d.can_identify)
                    && identifier(&i.request_id)
                    && (1..=10_000).contains(&i.remaining_ms),
            )?;
        }
        if let Some(r) = &e.scene {
            require(
                e.online
                    && s.timing.is_some()
                    && layout.is_some_and(|l| l.revision == r.revision)
                    && r.applied_at_ms.is_some() == e.applied_revision.is_some()
                    && r.applied_at_ms.is_none_or(|v| {
                        v > 0 && v <= MAX_SAFE && r.prepared && r.clock_uncertainty_ms.is_some()
                    })
                    && r.clock_uncertainty_ms.is_none_or(|v| v <= 1000),
            )?;
        }
    }
    Ok(())
}
