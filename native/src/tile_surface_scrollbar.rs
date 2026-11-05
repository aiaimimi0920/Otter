//! 已paint滚动条身份与捕获坐标；不持Art/scene/像素，不把捕获后的屏外坐标当命中资格。
use crate::{
    surface_presentation::Presentation,
    surface_scroll::Bar,
    tile_shared::{Frame, Key},
    tile_surface_owner::Art,
    tile_surface_wire::View,
};
#[derive(Clone)]
pub(crate) struct Field {
    pub bar: Bar,
    pub point: [f32; 2],
    pub instance: String,
    pixel: [i32; 2],
    key: Key,
    output: crate::tile_wire::Size,
    view: View,
    generation: u64,
    size: [u32; 2],
    placement: String,
    layout: [u8; 32],
}
#[cfg(test)]
#[path = "tile_surface_scrollbar_tests.rs"]
mod tests;
impl Field {
    pub(crate) fn at(frame: &Frame, art: &Art, p: &Presentation, pixel: [i32; 2]) -> Option<Self> {
        let (placement, uv) =
            crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
        if placement.source.id != art.view.instance_id {
            return None;
        }
        let point = [uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32];
        Some(Self {
            bar: p.scrollbar_at(point)?,
            point,
            instance: art.view.instance_id.clone(),
            pixel,
            key: frame.key.clone(),
            output: frame.size,
            view: art.view.clone(),
            generation: art.generation,
            size: art.size,
            placement: placement.placement_id.clone(),
            layout: p.layout_identity(),
        })
    }
    pub(crate) fn same(&self, other: &Self) -> bool {
        self.key == other.key
            && self.output == other.output
            && self.view == other.view
            && self.generation == other.generation
            && self.size == other.size
            && self.placement == other.placement
            && self.layout == other.layout
            && self.bar.id == other.bar.id
            && self.bar.axis == other.bar.axis
            && self.bar.track == other.bar.track
            && self.bar.clip == other.bar.clip
    }
    pub(crate) fn refreshed(&self, frame: &Frame, art: &Art, p: &Presentation) -> Option<Self> {
        Self::at(frame, art, p, self.pixel).filter(|next| self.same(next))
    }
    pub(crate) fn captured_point(&self, frame: &Frame, pixel: [i32; 2]) -> Option<[f32; 2]> {
        if self.key != frame.key || self.output != frame.size {
            return None;
        }
        let placement = frame.layout.placements.iter().find(|p| {
            p.placement_id == self.placement
                && p.interactive
                && p.source.kind == "surface"
                && p.source.id == self.instance
        })?;
        let wall =
            crate::tile_surface_hit::point(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
        let uv = crate::tile_surface_hit::source(placement, wall);
        let p = [uv[0] * self.size[0] as f32, uv[1] * self.size[1] as f32];
        p.iter().all(|v| v.is_finite()).then_some(p)
    }
    pub(crate) fn awaiting(
        &self,
        state: &crate::tile_shared::State,
        now: std::time::Instant,
    ) -> bool {
        !state.stopped
            && !state.expired
            && !state.frozen
            && !state.black
            && !state.identification.blocked()
            && state.desired.as_ref() == Some(&self.key)
            && state.until.is_some_and(|until| now < until)
            && state.clock.permits(state.activate_at, now)
            && state.presented_frame.is_none()
            && state.actions.states.get(&self.instance).is_some_and(|s| {
                s.view == self.view && s.generation == self.generation && s.size == self.size
            })
    }
}
