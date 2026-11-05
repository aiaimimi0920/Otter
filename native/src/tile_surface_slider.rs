//! Tile单手势Slider：复用canonical Range/编辑合同，只有真实paint命中才能建立拖动。
use crate::tile_surface_edit::Edit;
use crate::{
    surface_input_event::{Phase, Value},
    surface_presentation::Presentation,
    surface_scene::Kind,
    surface_slider::Range,
    tile_shared::Frame,
    tile_surface_actions::{Anchor, Command, Kind as Action},
    tile_surface_owner::Art,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone)]
pub(crate) struct Field {
    pub anchor: Anchor,
    pub node: String,
    key: crate::tile_shared::Key,
    range: Range,
    rect: [f32; 4],
    clip: [f32; 4],
    contract: [u8; 32],
    size: [u32; 2],
}
#[derive(Clone)]
pub(crate) struct Drag {
    pub field: Field,
    pub edit: Arc<Edit>,
    start: f64,
    pub value: f64,
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
    let node = crate::tile_surface_value::node(p.scene().root(), p.hit_test(point)?)?;
    if node.kind() != Kind::Slider
        || !node.visible()
        || node.props().get("disabled") == Some(&serde_json::Value::Bool(true))
    {
        return None;
    }
    let g = p.geometry().nodes().iter().find(|g| g.id() == node.id())?;
    let clip = g.clip();
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
        key: frame.key.clone(),
        range: Range::parse(node).ok()?,
        rect: [g.origin()[0], g.origin()[1], g.size()[0], g.size()[1]],
        clip: [clip.left, clip.top, clip.right, clip.bottom],
        contract: crate::surface_edit_contract::fingerprint(node).ok()?,
        size: art.size,
    })
}
impl Field {
    /// 只保留手势身份，不授新输入；旧呈现撤下到新帧实际paint前，Prepared/候选可先后到达。
    pub(crate) fn awaiting_frame(&self, state: &crate::tile_shared::State, now: Instant) -> bool {
        !state.stopped
            && !state.expired
            && !state.black
            && !state.frozen
            && !state.identification.blocked()
            && state.until.is_some_and(|t| now < t)
            && state.clock.permits(state.activate_at, now)
            && state.presented_frame.is_none()
            && state.desired.as_ref() == Some(&self.key)
            && state
                .actions
                .states
                .get(&self.anchor.view.instance_id)
                .is_some_and(|s| {
                    s.view == self.anchor.view && s.generation == self.anchor.generation
                })
    }
    pub fn same(&self, current: &Self) -> bool {
        self.key == current.key
            && self.anchor.view == current.anchor.view
            && self.anchor.generation == current.anchor.generation
            && self.anchor.placement == current.anchor.placement
            && self.node == current.node
            && self.contract == current.contract
            && self.rect == current.rect
            && self.clip == current.clip
            && self.size == current.size
    }
    pub fn value_at(&self, frame: &Frame, pixel: [i32; 2]) -> Option<f64> {
        if frame.key != self.key {
            return None;
        }
        let placement = frame
            .layout
            .placements
            .iter()
            .find(|p| p.placement_id == self.anchor.placement)?;
        let point =
            crate::tile_surface_hit::point(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
        let uv = crate::tile_surface_hit::source(placement, point);
        // 与嵌入绘制同一个5px拇指轨道；先变换到canonical坐标，再clamp/step一次。
        let left = self.rect[0] + 5.;
        let span = (self.rect[2] - 10.).max(0.);
        let fraction = if span == 0. {
            0.
        } else {
            (uv[0] * self.size[0] as f32 - left) / span
        };
        self.range.at_fraction(f64::from(fraction)).ok()
    }
}
impl Drag {
    pub fn local_start(&mut self, value: f64) {
        self.start = value;
        self.value = value;
    }
    pub fn new(field: Field) -> Self {
        let value = field.range.source();
        let edit = Edit::new(field.node.clone(), field.contract);
        Self {
            field,
            edit,
            start: value,
            value,
        }
    }
    pub fn sample(
        &mut self,
        frame: &Frame,
        pixel: [i32; 2],
    ) -> Result<Option<Command>, &'static str> {
        if !self.edit.active() {
            return Err("Art滑块手势已失效");
        }
        let value = self.field.value_at(frame, pixel).ok_or("Art滑块几何失效")?;
        if value == self.value {
            return Ok(None);
        }
        self.value = value;
        self.command(Phase::Input)
    }
    pub fn commit(&self) -> Result<Option<Command>, &'static str> {
        if self.start == self.value {
            return Ok(None);
        }
        self.command(Phase::Change)
    }
    pub(crate) fn command(&self, phase: Phase) -> Result<Option<Command>, &'static str> {
        if !self.edit.active() {
            return Err("Art滑块手势已失效");
        }
        let scene = crate::surface_scene::Scene::parse(&self.field.anchor.snapshot.scene)
            .map_err(|_| "Art滑块场景失效")?;
        let node = crate::tile_surface_value::node(scene.root(), &self.field.node)
            .ok_or("Art滑块已移除")?;
        let anchor = &self.field.anchor;
        let result = crate::surface_input_event::encode_identity(
            node,
            crate::surface_click_event::Identity {
                instance: &anchor.view.instance_id,
                attachment: &anchor.view.attachment_id,
                generation: anchor.generation,
            },
            anchor.snapshot.revision,
            phase,
            Value::Number(Some(self.value)),
        )
        .map_err(|_| "Art滑块编码失败")?;
        result
            .map(|(_, bytes)| {
                Ok(Command {
                    anchor: anchor.clone(),
                    kind: Action::Event(
                        serde_json::from_slice(&bytes).map_err(|_| "Art滑块编码失败")?,
                    ),
                    until: Instant::now() + Duration::from_millis(1500),
                    edit: Some(self.edit.clone()),
                })
            })
            .transpose()
    }
}
#[cfg(test)]
#[path = "tile_surface_slider_tests.rs"]
pub(crate) mod tests;
