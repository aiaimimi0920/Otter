//! 与同一布局提交的 CPU 命中快照；不持有 COM，不分发动作或自行取得输入授权。
use crate::surface_round_clip::Shape;
use crate::{
    surface_background::Background, surface_clip::Rect, surface_geometry::Geometry,
    surface_layout_tree::Tree, surface_length::Context, surface_scene::Node,
};
struct Entry {
    own: Option<Shape>,
    children: Option<Shape>,
}
pub(crate) struct Hits {
    entries: Vec<Entry>,
}
impl Hits {
    pub(crate) fn build(
        tree: &Tree<'_>,
        context: Context,
        font: impl Fn(&Node) -> Option<f32>,
    ) -> Self {
        let entries = tree
            .nodes()
            .iter()
            .map(|placed| {
                let local = Context {
                    font: font(placed.node).unwrap_or(context.font),
                    ..context
                };
                let bounds = Rect::from_box(placed.origin, placed.size);
                let shape = Background::new(placed, bounds, local);
                let own = shape
                    .as_ref()
                    .and_then(Background::rounded)
                    .map(Shape::from);
                let children = if crate::surface_overflow::computed(placed.node)
                    .iter()
                    .all(|axis| axis.clips())
                {
                    shape.as_ref().and_then(|shape| {
                        shape.padding_round(crate::surface_box::borders(placed.node, local))
                    })
                } else {
                    None
                };
                Entry { own, children }
            })
            .collect();
        Self { entries }
    }
    pub(crate) fn hit(&self, geometry: &Geometry, point: [f32; 2]) -> Option<usize> {
        if point.iter().any(|value| !value.is_finite())
            || geometry.nodes().len() != self.entries.len()
        {
            return None;
        }
        // 绘制最后的节点先命中；透明节点仍可接收输入，不能把 opacity 当 visible。
        for index in (0..self.entries.len()).rev() {
            let node = &geometry.nodes()[index];
            if !inside(node.clip(), point)
                || !self.entries[index]
                    .own
                    .as_ref()
                    .is_none_or(|shape| shape.contains(point))
            {
                continue;
            }
            let mut parent = node.parent();
            let mut visible = true;
            while let Some(index) = parent {
                if self.entries[index]
                    .children
                    .as_ref()
                    .is_some_and(|shape| !shape.contains(point))
                {
                    visible = false;
                    break;
                }
                parent = geometry.nodes()[index].parent();
            }
            if visible {
                return Some(index);
            }
        }
        None
    }
}
fn inside(rect: Rect, [x, y]: [f32; 2]) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}
