//! 覆盖式滚动条几何及绝对拖动；只改变本地偏移，不创建场景动作。
use super::*;
use crate::surface_clip::Rect;
const WIDTH: f32 = 10.;
const MIN_THUMB: f32 = 18.;
#[derive(Clone, Debug)]
pub(crate) struct Bar {
    pub id: String,
    pub axis: usize,
    pub track: Rect,
    pub thumb: Rect,
    pub clip: Rect,
    offset: f32,
    max: f32,
    page: f32,
}
#[derive(Clone)]
pub(crate) struct Drag {
    pub id: String,
    pub axis: usize,
    start: f32,
    offset: f32,
    ratio: f32,
}
fn contains(rect: Rect, p: [f32; 2]) -> bool {
    p[0] >= rect.left && p[0] < rect.right && p[1] >= rect.top && p[1] < rect.bottom
}
impl Bar {
    pub fn hit(&self, p: [f32; 2]) -> bool {
        contains(self.track.intersect(self.clip), p)
    }
    pub fn drag(&self, p: [f32; 2]) -> Option<Drag> {
        if !self.hit(p) || !contains(self.thumb, p) {
            return None;
        }
        let travel = if self.axis == 0 {
            self.track.right - self.track.left - self.thumb.right + self.thumb.left
        } else {
            self.track.bottom - self.track.top - self.thumb.bottom + self.thumb.top
        };
        (travel > 0.).then(|| Drag {
            id: self.id.clone(),
            axis: self.axis,
            start: p[self.axis],
            offset: self.offset,
            ratio: self.max / travel,
        })
    }
    pub fn page_value(&self, p: [f32; 2]) -> f32 {
        self.offset + f32::from(self.page_direction(p)) * self.page
    }
    pub fn page_direction(&self, p: [f32; 2]) -> i8 {
        if !self.hit(p) || contains(self.thumb, p) {
            return 0;
        }
        let start = if self.axis == 0 {
            self.thumb.left
        } else {
            self.thumb.top
        };
        if p[self.axis] < start {
            -1
        } else {
            1
        }
    }
}
impl Drag {
    pub fn value(&self, p: [f32; 2]) -> f32 {
        self.offset + (p[self.axis] - self.start) * self.ratio
    }
}
impl Scroll {
    /// 常驻覆盖式条不改变布局；双轴保留角区，hidden 轴没有条也不接受拖动。
    pub(crate) fn bars(&self, geometry: &Geometry) -> Vec<Bar> {
        let mut bars = Vec::new();
        for node in geometry.nodes() {
            let Some(entry) = self.nodes.get(node.id()) else {
                continue;
            };
            let visible: [bool; 2] = std::array::from_fn(|a| entry.wheel[a] && entry.max[a] > 0.);
            for axis in 0..2 {
                if !visible[axis] {
                    continue;
                }
                let cross = 1 - axis;
                let mut origin: [f32; 2] =
                    std::array::from_fn(|a| node.origin()[a] + entry.border.start()[a]);
                let length = (entry.page[axis] - if visible[cross] { WIDTH } else { 0. }).max(0.);
                let thickness = WIDTH.min(entry.page[cross]);
                if length <= 0. || thickness <= 0. {
                    continue;
                }
                origin[cross] += entry.page[cross] - thickness;
                let mut size = [0.; 2];
                size[axis] = length;
                size[cross] = thickness;
                let track = Rect::from_box(origin, size);
                let clip = node.clip();
                if track.intersect(clip).empty() {
                    continue;
                }
                let thumb_length = (length * entry.page[axis]
                    / (entry.page[axis] + entry.max[axis]))
                    .max(MIN_THUMB)
                    .min(length * 0.8);
                origin[axis] += (length - thumb_length) * entry.offset[axis] / entry.max[axis];
                size[axis] = thumb_length;
                bars.push(Bar {
                    id: node.id().to_owned(),
                    axis,
                    track,
                    thumb: Rect::from_box(origin, size),
                    clip,
                    offset: entry.offset[axis],
                    max: entry.max[axis],
                    page: entry.page[axis],
                });
            }
        }
        bars
    }
    pub(crate) fn set_bar(&mut self, id: &str, axis: usize, value: f32) -> bool {
        if axis > 1 || !value.is_finite() {
            return false;
        }
        let Some(entry) = std::sync::Arc::make_mut(&mut self.nodes).get_mut(id) else {
            return false;
        };
        if !entry.wheel[axis] {
            return false;
        }
        let value = value.clamp(0., entry.max[axis]);
        if value == entry.offset[axis] {
            return false;
        }
        entry.offset[axis] = value;
        true
    }
}
