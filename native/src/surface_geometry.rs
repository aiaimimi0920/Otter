//! 可长期持有的布局几何；从成功布局树复制，不借用场景或重新执行测量。
use crate::{
    surface_clip::{self, Rect},
    surface_layout_tree::Tree,
    surface_scene::Kind,
};

#[derive(Debug)]
pub struct NodeGeometry {
    id: String,
    kind: Kind,
    parent: Option<usize>,
    origin: [f32; 2],
    size: [f32; 2],
    clip: Rect,
}
impl NodeGeometry {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }
    pub fn origin(&self) -> [f32; 2] {
        self.origin
    }
    pub fn size(&self) -> [f32; 2] {
        self.size
    }
    /// 节点自身矩形与祖先裁剪的交集；若输入树已滚动，此处为滚动后的几何。
    pub fn clip(&self) -> Rect {
        self.clip
    }
}
#[derive(Debug)]
pub struct Geometry {
    nodes: Vec<NodeGeometry>,
}
impl Geometry {
    pub fn from_tree(tree: &Tree<'_>) -> Self {
        Self::with_border(tree, |_| 0_f32.into())
    }
    pub(crate) fn with_border(
        tree: &Tree<'_>,
        border: impl Fn(&crate::surface_scene::Node) -> crate::surface_box::Insets,
    ) -> Self {
        let clips = surface_clip::ancestors_with_border(tree, border);
        Self {
            nodes: tree
                .nodes()
                .iter()
                .zip(clips)
                .map(|(placed, clip)| NodeGeometry {
                    id: placed.node.id().to_owned(),
                    kind: placed.node.kind(),
                    parent: placed.parent,
                    origin: placed.origin,
                    size: placed.size,
                    clip: clip.intersect(Rect::from_box(placed.origin, placed.size)),
                })
                .collect(),
        }
    }
    /// 保留树的父先子后顺序和所有节点，包括零面积与完全被裁剪的节点。
    pub fn nodes(&self) -> &[NodeGeometry] {
        &self.nodes
    }
}
