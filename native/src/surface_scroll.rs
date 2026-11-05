//! 有界本地滚动状态；布局完成后平移后代，嵌套裁剪边界阻止内部溢出扩散。
use crate::{
    surface_geometry::Geometry,
    surface_layout_tree::Tree,
    surface_overflow::{self, Mode},
};
use std::collections::BTreeMap;
#[path = "surface_scrollbar.rs"]
mod scrollbar;
#[cfg(test)]
#[path = "surface_scroll_tests.rs"]
mod tests;
pub(crate) use scrollbar::{Bar, Drag};
#[path = "surface_scroll_keyboard.rs"]
mod keyboard;
pub(crate) use keyboard::Command;

#[derive(Clone, Default, PartialEq)]
pub(crate) struct Scroll {
    nodes: std::sync::Arc<BTreeMap<String, Entry>>,
}
#[derive(Clone, PartialEq)]
struct Entry {
    offset: [f32; 2],
    max: [f32; 2],
    page: [f32; 2],
    border: crate::surface_box::Insets,
    wheel: [bool; 2],
}
impl Scroll {
    /// 快照共享只读偏移；写入走COW。按owner保守计费，不借Arc共享绕过动作队列预算。
    pub(crate) fn bytes(&self) -> usize {
        256 + self.nodes.keys().map(|id| id.len() + 256).sum::<usize>()
    }
    /// 从未平移的布局计算范围；只保留当前场景存在的可滚动节点。
    pub(crate) fn apply(
        &mut self,
        tree: &mut Tree<'_>,
        border: impl Fn(&crate::surface_scene::Node) -> crate::surface_box::Insets,
    ) {
        let nodes = tree.nodes();
        let mut ends: Vec<[f32; 2]> = nodes
            .iter()
            .map(|p| {
                let b = border(p.node);
                std::array::from_fn(|a| p.origin[a] + p.size[a] - [b.right, b.bottom][a])
            })
            .collect();
        let mut next = BTreeMap::new();
        for i in (0..nodes.len()).rev() {
            let p = &nodes[i];
            let border = border(p.node);
            let modes = surface_overflow::computed(p.node);
            if modes.iter().any(|mode| mode.clips()) {
                let max = std::array::from_fn(|a| {
                    if modes[a].clips() {
                        (ends[i][a] - p.origin[a] - p.size[a] + [border.right, border.bottom][a])
                            .max(0.)
                    } else {
                        0.
                    }
                });
                let old = self.nodes.get(p.node.id()).map_or([0.; 2], |e| e.offset);
                next.insert(
                    p.node.id().to_owned(),
                    Entry {
                        offset: std::array::from_fn(|a| old[a].clamp(0., max[a])),
                        max,
                        page: crate::surface_box::inner(p.size, border),
                        border,
                        wheel: modes.map(|mode| mode == Mode::Auto),
                    },
                );
            }
            if let Some(parent) = p.parent {
                for a in 0..2 {
                    let end = if modes[a].clips() {
                        p.origin[a] + p.size[a]
                    } else {
                        ends[i][a].max(p.origin[a] + p.size[a])
                    };
                    ends[parent][a] = ends[parent][a].max(end);
                }
            }
        }
        self.nodes = std::sync::Arc::new(next);
        let mut shifts = Vec::<[f32; 2]>::with_capacity(nodes.len());
        for p in tree.nodes_mut() {
            let shift = p.parent.map_or([0.; 2], |parent| shifts[parent]);
            for (origin, delta) in p.origin.iter_mut().zip(shift) {
                *origin -= delta;
            }
            let own = self.nodes.get(p.node.id()).map_or([0.; 2], |e| e.offset);
            shifts.push(std::array::from_fn(|a| shift[a] + own[a]));
        }
    }
    /// 从最内层命中节点向祖先寻找可消费方向；到边界才交给外层，不双重消费。
    pub(crate) fn wheel(
        &mut self,
        geometry: &Geometry,
        hit: &str,
        axis: usize,
        delta: f32,
        page: bool,
    ) -> bool {
        if axis > 1 || !delta.is_finite() || delta == 0. {
            return false;
        }
        let mut index = geometry.nodes().iter().position(|node| node.id() == hit);
        while let Some(i) = index {
            let node = &geometry.nodes()[i];
            if let Some(entry) = std::sync::Arc::make_mut(&mut self.nodes).get_mut(node.id()) {
                if !entry.wheel[axis] {
                    index = node.parent();
                    continue;
                }
                let distance = if page {
                    delta * entry.page[axis]
                } else {
                    delta
                };
                let value = (entry.offset[axis] + distance).clamp(0., entry.max[axis]);
                if value != entry.offset[axis] {
                    entry.offset[axis] = value;
                    return true;
                }
            }
            index = node.parent();
        }
        false
    }
    /// 键盘聚焦允许滚动 hidden 轴，但不能赋予该轴滚轮资格。
    pub(crate) fn reveal(&mut self, geometry: &Geometry, id: &str) -> bool {
        let Some(target) = geometry.nodes().iter().find(|node| node.id() == id) else {
            return false;
        };
        self.reveal_region(geometry, id, target.origin(), target.size())
    }
    /// 显露节点内的局部目标（例如选区端点），而不是把整个长文本框当目标。
    pub(crate) fn reveal_region(
        &mut self,
        geometry: &Geometry,
        id: &str,
        origin: [f32; 2],
        size: [f32; 2],
    ) -> bool {
        self.reveal_region_in(geometry, id, origin, size, None)
    }
    pub(crate) fn reveal_region_in(
        &mut self,
        geometry: &Geometry,
        id: &str,
        mut origin: [f32; 2],
        size: [f32; 2],
        viewport: Option<crate::surface_clip::Rect>,
    ) -> bool {
        if origin.iter().chain(size.iter()).any(|v| !v.is_finite()) || size.iter().any(|v| *v <= 0.)
        {
            return false;
        }
        let Some(target) = geometry.nodes().iter().find(|node| node.id() == id) else {
            return false;
        };
        let mut parent = target.parent();
        let mut changed = false;
        while let Some(index) = parent {
            let node = &geometry.nodes()[index];
            if let Some(entry) = std::sync::Arc::make_mut(&mut self.nodes).get_mut(node.id()) {
                for axis in 0..2 {
                    let start = node.origin()[axis] + entry.border.start()[axis];
                    let end = start + entry.page[axis];
                    let (start, end) = viewport.map_or((start, end), |v| {
                        (
                            start.max([v.left, v.top][axis]),
                            end.min([v.right, v.bottom][axis]),
                        )
                    });
                    if start >= end {
                        continue;
                    }
                    let far = origin[axis] + size[axis];
                    let delta = nearest(origin[axis], far, start, end);
                    let next = (entry.offset[axis] + delta).clamp(0., entry.max[axis]);
                    origin[axis] -= next - entry.offset[axis];
                    changed |= next != entry.offset[axis];
                    entry.offset[axis] = next;
                }
            }
            parent = node.parent();
        }
        changed
    }
}
fn nearest(start: f32, end: f32, low: f32, high: f32) -> f32 {
    if (start >= low && end <= high) || (start < low && end > high) {
        return 0.;
    }
    if (start < low && end - start <= high - low) || (end > high && end - start > high - low) {
        start - low
    } else {
        end - high
    }
}
