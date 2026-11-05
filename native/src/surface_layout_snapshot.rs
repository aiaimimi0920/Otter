//! 已测量布局的有界拥有型快照；不保存指向 Scene 的自引用或裸指针。
use super::*;
pub(crate) struct Snapshot {
    viewport: [f32; 2],
    nodes: Vec<Saved>,
}
struct Saved {
    id: String,
    kind: Kind,
    containing_width: f32,
    grid_cell: Option<[f32; 2]>,
    origin: [f32; 2],
    size: [f32; 2],
    parent: Option<usize>,
}
impl Snapshot {
    pub(crate) fn new(tree: &Tree<'_>) -> Self {
        Self {
            viewport: tree.viewport,
            nodes: tree
                .nodes
                .iter()
                .map(|p| Saved {
                    id: p.node.id().to_owned(),
                    kind: p.node.kind(),
                    containing_width: p.containing_width,
                    grid_cell: p.grid_cell,
                    origin: p.origin,
                    size: p.size,
                    parent: p.parent,
                })
                .collect(),
        }
    }
    /// 必须与原树相同的可见节点身份、类型和父序；不跨新场景盲目套用旧尺寸。
    pub(crate) fn restore<'a>(&self, scene: &'a Scene) -> Option<Tree<'a>> {
        fn visit<'a>(
            node: &'a Node,
            parent: Option<usize>,
            out: &mut Vec<(&'a Node, Option<usize>)>,
        ) {
            if !node.visible() {
                return;
            }
            let index = out.len();
            out.push((node, parent));
            for child in node.visible_children() {
                visit(child, Some(index), out);
            }
        }
        let mut visible = Vec::with_capacity(scene.node_count());
        visit(scene.root(), None, &mut visible);
        if visible.len() != self.nodes.len() {
            return None;
        }
        let nodes = self
            .nodes
            .iter()
            .zip(visible)
            .map(|(saved, (node, parent))| {
                if saved.id != node.id() || saved.kind != node.kind() || saved.parent != parent {
                    return None;
                }
                Some(Placed {
                    node,
                    parent,
                    origin: saved.origin,
                    size: saved.size,
                    containing_width: saved.containing_width,
                    grid_cell: saved.grid_cell,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Tree {
            viewport: self.viewport,
            nodes,
        })
    }
}
