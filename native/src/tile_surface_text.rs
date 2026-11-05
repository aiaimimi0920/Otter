//! Tile文本字段命中/稳定合同与原生编辑层几何；不构造Surface Binding或持有窗口资源。
use crate::{
    surface_presentation::{ControlFont, Presentation},
    surface_scene::Kind,
    tile_shared::{Frame, Key},
    tile_surface_actions::Anchor,
    tile_surface_owner::Art,
};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Field {
    pub anchor: Anchor,
    pub node: String,
    pub contract: [u8; 32],
    pub kind: Kind,
    key: Key,
    pub rect: [i32; 4],
    pub font: ControlFont,
    size: [u32; 2],
    geometry: ([f32; 2], [f32; 2], [f32; 4]),
    layout: [u8; 32],
}
#[derive(Clone)]
pub(crate) struct Preview {
    pub field: Field,
    pub value: std::rc::Rc<str>,
}
impl PartialEq for Preview {
    fn eq(&self, other: &Self) -> bool {
        self.field.same(&other.field)
            && self.value == other.value
            && Arc::ptr_eq(&self.field.anchor.snapshot, &other.field.anchor.snapshot)
    }
}
pub(crate) fn hit(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    pixel: [i32; 2],
) -> Option<Field> {
    let (placement, uv) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    if placement.source.id != art.view.instance_id {
        return None;
    }
    let point = [uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32];
    let node = p.scene().root().visible_node(p.hit_test(point)?)?;
    if !matches!(node.kind(), Kind::Input | Kind::Textarea | Kind::Number)
        || node.props().get("disabled") == Some(&serde_json::Value::Bool(true))
    {
        return None;
    }
    let geometry = p.geometry().nodes().iter().find(|g| g.id() == node.id())?;
    let clip = geometry.clip();
    let bounds =
        crate::surface_clip::Rect::from_box(geometry.origin(), geometry.size()).intersect(clip);
    if bounds.empty() {
        return None;
    }
    let mapped = crate::tile_surface_geometry::bounds(frame, art, placement, bounds)?;
    let rect = [
        mapped[0].floor(),
        mapped[1].floor(),
        mapped[2].ceil(),
        mapped[3].ceil(),
    ];
    let mut font = p.control_font(node.id())?.clone();
    let a = crate::tile_surface_geometry::map(frame, art, placement, geometry.origin())?;
    let b = crate::tile_surface_geometry::map(
        frame,
        art,
        placement,
        [geometry.origin()[0], geometry.origin()[1] + 1.],
    )?;
    font.size = (f64::from(font.size) * ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt())
        .clamp(1., 512.) as f32;
    Some(Field {
        anchor: Anchor {
            view: art.view.clone(),
            generation: art.generation,
            snapshot: art.snapshot.clone(),
            placement: placement.placement_id.clone(),
            pixel,
            scroll: p.scroll_state(),
        },
        node: node.id().into(),
        contract: crate::surface_edit_contract::fingerprint(node).ok()?,
        kind: node.kind(),
        key: frame.key.clone(),
        rect: rect.map(|v| v as i32),
        font,
        size: art.size,
        geometry: (
            geometry.origin(),
            geometry.size(),
            [clip.left, clip.top, clip.right, clip.bottom],
        ),
        layout: p.layout_identity(),
    })
}
impl Field {
    /// 只保留已关闭编辑器的草稿/票据；不把屏外字段伪装成新的可输入命中。
    pub(crate) fn retained(&self, frame: &Frame, art: &Art, p: &Presentation) -> Option<Self> {
        if self.key != frame.key
            || !self.matches_art(art, p)
            || self.layout != p.layout_identity()
            || !frame.layout.placements.iter().any(|v| {
                v.placement_id == self.anchor.placement
                    && v.interactive
                    && v.source.kind == "surface"
                    && v.source.id == art.view.instance_id
            })
        {
            return None;
        }
        let mut next = self.clone();
        next.anchor.snapshot = art.snapshot.clone();
        // 保留原事件激活时的滚动坐标；重新打开时由真实hit替换整个Field。
        Some(next)
    }
    pub fn current(&self, frame: &Frame) -> bool {
        self.key == frame.key
            && frame
                .arts
                .get(&self.anchor.view.instance_id)
                .is_some_and(|a| {
                    a.view == self.anchor.view
                        && a.generation == self.anchor.generation
                        && Arc::ptr_eq(&a.snapshot, &self.anchor.snapshot)
                })
    }
    pub fn matches_art(&self, art: &Art, p: &Presentation) -> bool {
        self.anchor.view == art.view
            && self.anchor.generation == art.generation
            && self.size == art.size
            && p.scene()
                .root()
                .visible_node(&self.node)
                .and_then(|n| crate::surface_edit_contract::fingerprint(n).ok())
                == Some(self.contract)
    }
    pub fn same(&self, other: &Self) -> bool {
        self.key == other.key
            && self.anchor.view == other.anchor.view
            && self.anchor.generation == other.anchor.generation
            && self.anchor.placement == other.anchor.placement
            && self.node == other.node
            && self.kind == other.kind
            && self.contract == other.contract
            && self.size == other.size
            && self.geometry == other.geometry
            && self.rect == other.rect
    }
    pub fn awaiting(&self, state: &crate::tile_shared::State, now: std::time::Instant) -> bool {
        !state.stopped
            && !state.expired
            && !state.frozen
            && !state.black
            && !state.identification.blocked()
            && state.until.is_some_and(|t| now < t)
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
#[cfg(test)]
#[path = "tile_surface_text_tests.rs"]
mod tests;
