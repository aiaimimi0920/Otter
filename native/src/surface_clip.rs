//! 从布局树累计祖先裁剪；仅改变绘制可见区域，不修改尺寸或测量反馈。
use crate::surface_layout_tree::Tree;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}
impl Rect {
    pub fn from_box(origin: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            left: origin[0],
            top: origin[1],
            right: origin[0] + size[0],
            bottom: origin[1] + size[1],
        }
    }
    pub fn intersect(self, other: Self) -> Self {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        Self {
            left,
            top,
            right: self.right.min(other.right).max(left),
            bottom: self.bottom.min(other.bottom).max(top),
        }
    }
    pub fn empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }
}
/// Tree 保证父索引先于子项。根宿主始终裁剪，普通容器按 overflow 计算值裁剪。
pub fn ancestors(tree: &Tree<'_>) -> Vec<Rect> {
    ancestors_with_border(tree, |_| 0_f32.into())
}
/// 边框宽度由已计算字号的布局上下文提供；子内容裁剪到 padding box。
pub(crate) fn ancestors_with_border(
    tree: &Tree<'_>,
    border: impl Fn(&crate::surface_scene::Node) -> crate::surface_box::Insets,
) -> Vec<Rect> {
    let mut visible = Vec::with_capacity(tree.nodes().len());
    let mut children = Vec::with_capacity(tree.nodes().len());
    for placed in tree.nodes() {
        let inherited = placed
            .parent
            .map(|index| children[index])
            .unwrap_or(Rect::from_box([0.; 2], tree.viewport()));
        visible.push(inherited);
        let inset = border(placed.node);
        let bounds = Rect::from_box(
            std::array::from_fn(|a| placed.origin[a] + inset.start()[a]),
            crate::surface_box::inner(placed.size, inset),
        );
        let axes = crate::surface_overflow::computed(placed.node);
        let mut next = inherited;
        if axes[0].clips() {
            next.left = next.left.max(bounds.left);
            next.right = next.right.min(bounds.right).max(next.left);
        }
        if axes[1].clips() {
            next.top = next.top.max(bounds.top);
            next.bottom = next.bottom.min(bounds.bottom).max(next.top);
        }
        children.push(next);
    }
    visible
}
#[cfg(test)]
#[path = "surface_clip_tests.rs"]
mod tests;
