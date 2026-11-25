//! Layout geometry safe to retain long-term; copied from a successful layout tree, without borrowing the scene or rerunning measurement.
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
    /// Intersection of the node's rectangle with ancestor clips; if the input tree is scrolled, this is post-scroll geometry.
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
    pub fn with_border(
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
    /// Preserve the tree's parent-before-child order and all nodes, including zero-area and fully clipped nodes.
    pub fn nodes(&self) -> &[NodeGeometry] {
        &self.nodes
    }
}
