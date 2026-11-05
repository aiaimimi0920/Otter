//! Tile键盘焦点身份及有界可命中像素搜索；不把控件中心或滚轮方向当可见性证据。
use crate::{
    surface_presentation::Presentation,
    surface_scene::Kind,
    tile_shared::{Frame, Key},
    tile_surface_actions::Anchor,
    tile_surface_owner::Art,
    tile_wire::Placement,
};
use std::sync::Arc;
#[path = "tile_surface_focus_search.rs"]
mod search;
#[derive(Clone)]
pub(crate) struct Focus {
    pub anchor: Anchor,
    pub node: String,
    pub kind: Kind,
    pub contract: [u8; 32],
    pub rect: [i32; 4],
    key: Key,
    size: [u32; 2],
    geometry: ([f32; 2], [f32; 2], [f32; 4]),
    layout: [u8; 32],
}
impl Focus {
    /// 本地草稿身份忽略滚动平移，但仍核完整未滚动布局；不可据此激活隐藏字段。
    pub(crate) fn same_local(&self, other: &Self) -> bool {
        self.key == other.key
            && self.anchor.view == other.anchor.view
            && self.anchor.generation == other.anchor.generation
            && self.anchor.placement == other.anchor.placement
            && self.node == other.node
            && self.kind == other.kind
            && self.contract == other.contract
            && self.size == other.size
            && self.layout == other.layout
            && (self.anchor.scroll != other.anchor.scroll
                || self.rect == other.rect && self.geometry == other.geometry)
    }
    pub(crate) fn retained(&self, frame: &Frame, art: &Art, p: &Presentation) -> Option<Self> {
        if self.key != frame.key
            || self.anchor.view != art.view
            || self.anchor.generation != art.generation
            || self.size != art.size
            || self.layout != p.layout_identity()
            || !p.can_focus(&self.node)
            || p.scene()
                .root()
                .visible_node(&self.node)
                .and_then(|n| crate::surface_edit_contract::fingerprint(n).ok())
                != Some(self.contract)
        {
            return None;
        }
        let placement = frame.layout.placements.iter().find(|v| {
            v.placement_id == self.anchor.placement
                && v.interactive
                && v.source.kind == "surface"
                && v.source.id == art.view.instance_id
        })?;
        let geometry = p.geometry().nodes().iter().find(|g| g.id() == self.node)?;
        let mut next = self.clone();
        next.anchor.snapshot = art.snapshot.clone();
        next.anchor.scroll = p.scroll_state();
        // 被动保留也更新真实几何；屏外空矩形不授输入资格，部分显露后再命中则保留本地值。
        next.geometry = (
            geometry.origin(),
            geometry.size(),
            [
                geometry.clip().left,
                geometry.clip().top,
                geometry.clip().right,
                geometry.clip().bottom,
            ],
        );
        next.rect = crate::tile_surface_geometry::bounds(frame, art, placement, geometry.clip())
            .map_or([0; 4], |r| {
                [
                    r[0].floor() as i32,
                    r[1].floor() as i32,
                    r[2].ceil() as i32,
                    r[3].ceil() as i32,
                ]
            });
        Some(next)
    }
    pub fn same(&self, other: &Self) -> bool {
        self.key == other.key
            && self.anchor.view == other.anchor.view
            && self.anchor.generation == other.anchor.generation
            && self.anchor.placement == other.anchor.placement
            && self.node == other.node
            && self.kind == other.kind
            && self.contract == other.contract
            && self.rect == other.rect
            && self.size == other.size
            && self.geometry == other.geometry
    }
    pub fn awaiting(&self, state: &crate::tile_shared::State, now: std::time::Instant) -> bool {
        !state.stopped
            && !state.expired
            && !state.frozen
            && !state.black
            && !state.identification.blocked()
            && state.until.is_some_and(|v| now < v)
            && state.clock.permits(state.activate_at, now)
            && state.desired.as_ref() == Some(&self.key)
            && state.presented_frame.is_none()
            && state
                .actions
                .states
                .get(&self.anchor.view.instance_id)
                .is_some_and(|s| {
                    s.view == self.anchor.view && s.generation == self.anchor.generation
                })
    }
}
pub(crate) fn at(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    pixel: [i32; 2],
) -> Option<Focus> {
    let (placement, uv) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    if placement.source.id != art.view.instance_id {
        return None;
    }
    let node = p.hit_test([uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32])?;
    if !p.focusable(node) {
        return None;
    }
    let n = p.scene().root().visible_node(node)?;
    let geometry = p.geometry().nodes().iter().find(|g| g.id() == node)?;
    let rect = crate::tile_surface_geometry::bounds(frame, art, placement, geometry.clip())?;
    Some(Focus {
        anchor: Anchor {
            view: art.view.clone(),
            generation: art.generation,
            snapshot: art.snapshot.clone(),
            placement: placement.placement_id.clone(),
            pixel,
            scroll: p.scroll_state(),
        },
        node: node.into(),
        kind: n.kind(),
        contract: crate::surface_edit_contract::fingerprint(n).ok()?,
        rect: [
            rect[0].floor() as i32,
            rect[1].floor() as i32,
            rect[2].ceil() as i32,
            rect[3].ceil() as i32,
        ],
        key: frame.key.clone(),
        size: art.size,
        geometry: (
            geometry.origin(),
            geometry.size(),
            [
                geometry.clip().left,
                geometry.clip().top,
                geometry.clip().right,
                geometry.clip().bottom,
            ],
        ),
        layout: p.layout_identity(),
    })
}
/// 整数像素区域二分，最终每个候选都经Tile最上层与Surface圆角/scrollbar双重回验。
/// 预算耗尽是明确失败，不将尚未搜索的控件谎报不可见；只在导航时执行，不常驻采样。
pub(crate) fn find(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    placement: &Placement,
    node: &str,
    budget: &mut usize,
) -> Result<Option<Focus>, &'static str> {
    if !placement.interactive || placement.source.kind != "surface" || !p.focusable(node) {
        return Ok(None);
    }
    let Some(g) = p.geometry().nodes().iter().find(|g| g.id() == node) else {
        return Ok(None);
    };
    let Some(r) = crate::tile_surface_geometry::bounds(frame, art, placement, g.clip()) else {
        return Ok(None);
    };
    let r = [
        (r[0] - 0.5).ceil() as i32,
        (r[1] - 0.5).ceil() as i32,
        (r[2] - 0.5).ceil() as i32,
        (r[3] - 0.5).ceil() as i32,
    ];
    let mut pending = vec![r];
    while let Some([l, t, r, b]) = pending.pop() {
        if l >= r || t >= b {
            continue;
        }
        if *budget == 0 {
            return Err("Art键盘可见区域搜索预算不足");
        }
        *budget -= 1;
        if search::covered(frame, art, p, placement, node, [l, t, r, b]) {
            continue;
        }
        let pixel = [l + (r - l - 1) / 2, t + (b - t - 1) / 2];
        if let Some(f) = at(frame, art, p, pixel)
            .filter(|f| f.node == node && f.anchor.placement == placement.placement_id)
        {
            return Ok(Some(f));
        }
        if r - l >= b - t && r - l > 1 {
            let m = l + (r - l) / 2;
            pending.push([m, t, r, b]);
            pending.push([l, t, m, b]);
        } else if b - t > 1 {
            let m = t + (b - t) / 2;
            pending.push([l, m, r, b]);
            pending.push([l, t, r, m]);
        }
    }
    Ok(None)
}
#[cfg(test)]
#[path = "tile_surface_focus_tests.rs"]
mod tests;
