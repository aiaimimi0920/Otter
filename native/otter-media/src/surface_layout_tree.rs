//! Bounded scene-tree layout traversal; the host supplies measurement, geometry commits once, and failed partial trees are not delivered.
use crate::{
    surface_container,
    surface_length::Context,
    surface_scene::{Kind, Node, Scene, MAX_DEPTH, MAX_NODES},
};
#[path = "surface_layout_snapshot.rs"]
pub mod snapshot;
pub use snapshot::Snapshot;
#[derive(Clone, Copy, Debug)]
pub struct Intrinsic {
    pub size: [f32; 2],
    pub baseline: Option<f32>,
}
pub trait Measure {
    type Error;
    fn font_size(&self, _node: &Node) -> Option<f32> {
        None
    }
    /// available is the parent's current allocation, not the final child size; the measurement owner handles subsequent text reflow.
    fn measure(
        &mut self,
        node: &Node,
        available: [f32; 2],
        context: Context,
    ) -> Result<Intrinsic, Self::Error>;
}
#[derive(Debug)]
pub struct Placed<'a> {
    /// Percentage padding uses containing-block width, not the node's own width or the host cqw query width.
    pub containing_width: f32,
    /// A Grid item's containing block is its cell, not the whole parent content area; image percentages and aspect-ratio feedback reuse it.
    pub grid_cell: Option<[f32; 2]>,
    pub node: &'a Node,
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub parent: Option<usize>,
}
#[derive(Debug)]
pub struct Tree<'a> {
    viewport: [f32; 2],
    nodes: Vec<Placed<'a>>,
}
#[derive(Debug)]
pub enum Error {
    Input,
    Limit,
    Measurement,
    Unsupported,
    Container(surface_container::Error),
}
impl<'a> Tree<'a> {
    pub fn build(
        scene: &'a Scene,
        size: [f32; 2],
        context: Context,
        measure: &mut impl Measure,
    ) -> Result<Self, Error> {
        if size
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
        {
            return Err(Error::Input);
        }
        if crate::surface_length::Length::Scalar(0., crate::surface_length::Unit::Px)
            .resolve(context)
            .is_none()
        {
            return Err(Error::Input);
        }
        let mut tree = Self {
            viewport: size,
            nodes: Vec::with_capacity(scene.node_count()),
        };
        if !scene.root().visible() {
            return Ok(tree);
        }
        let root_context = Context {
            font: measure.font_size(scene.root()).unwrap_or(context.font),
            ..context
        };
        let intrinsic = measure
            .measure(scene.root(), size, context)
            .map_err(|_| Error::Measurement)?;
        let root_size =
            crate::surface_root_box::resolve(scene.root(), size, intrinsic.size, root_context)
                .ok_or(Error::Input)?;
        tree.visit(
            Placed {
                containing_width: size[0],
                grid_cell: None,
                node: scene.root(),
                origin: crate::surface_root_box::origin(
                    scene.root(),
                    size,
                    root_size,
                    root_context,
                ),
                size: root_size,
                parent: None,
            },
            context,
            measure,
            1,
        )?;
        Ok(tree)
    }
    pub fn nodes(&self) -> &[Placed<'a>] {
        &self.nodes
    }
    pub fn nodes_mut(&mut self) -> &mut [Placed<'a>] {
        &mut self.nodes
    }
    pub fn viewport(&self) -> [f32; 2] {
        self.viewport
    }
    fn visit(
        &mut self,
        placed: Placed<'a>,
        context: Context,
        measure: &mut impl Measure,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > MAX_DEPTH || self.nodes.len() >= MAX_NODES {
            return Err(Error::Limit);
        }
        if placed
            .origin
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        {
            return Err(Error::Input);
        }
        let node = placed.node;
        let origin = placed.origin;
        let size = placed.size;
        let containing_width = placed.containing_width;
        let local = Context {
            font: measure.font_size(node).unwrap_or(context.font),
            ..context
        };
        let content = crate::surface_box::inner(
            size,
            crate::surface_box::inset(node, containing_width, local),
        );
        let padding_box = crate::surface_box::inner(size, crate::surface_box::borders(node, local));
        let available = |child: &Node| {
            if child
                .layout()
                .get("position")
                .is_some_and(|v| v == "absolute")
            {
                padding_box
            } else {
                content
            }
        };
        let parent = self.nodes.len();
        self.nodes.push(placed);
        if node.visible_children().next().is_none() {
            return Ok(());
        }
        if !matches!(
            node.kind(),
            Kind::Row | Kind::Column | Kind::View | Kind::Scroll | Kind::Stack | Kind::Grid
        ) {
            return Err(Error::Unsupported);
        }
        let mut measured = node
            .visible_children()
            .map(|child| {
                let value = measure
                    .measure(child, available(child), context)
                    .map_err(|_| Error::Measurement)?;
                Ok(surface_container::Measured {
                    id: child.id(),
                    size: value.size,
                    baseline: value.baseline,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut grid_cells = None;
        let boxes = if node.kind() == Kind::Grid {
            let initial = crate::surface_grid::layout(
                node,
                &measured,
                size,
                context,
                &|n| measure.font_size(n),
                containing_width,
                true,
            )
            .map_err(|_| Error::Unsupported)?;
            for ((child, value), cell) in node
                .visible_children()
                .zip(&mut measured)
                .zip(initial.cells)
            {
                let next = measure
                    .measure(child, cell, context)
                    .map_err(|_| Error::Measurement)?;
                value.size[1] = next.size[1];
                value.baseline = next.baseline;
            }
            let final_layout = crate::surface_grid::layout(
                node,
                &measured,
                size,
                context,
                &|n| measure.font_size(n),
                containing_width,
                true,
            )
            .map_err(|_| Error::Unsupported)?;
            grid_cells = Some(final_layout.cells);
            final_layout.boxes
        } else {
            surface_container::arrange_in(
                node,
                &measured,
                size,
                context,
                &|node| measure.font_size(node),
                containing_width,
            )
            .map_err(Error::Container)?
        };
        // Fix the parent result first, then traverse depth-first to preserve stable drawing order and parent indices; no fixed-point retries.
        for (index, child) in boxes.into_iter().enumerate() {
            let cell = grid_cells.as_ref().map(|cells| cells[index]);
            self.visit(
                Placed {
                    containing_width: cell.unwrap_or_else(|| available(child.node))[0],
                    grid_cell: cell,
                    node: child.node,
                    origin: [origin[0] + child.origin[0], origin[1] + child.origin[1]],
                    size: child.size,
                    parent: Some(parent),
                },
                context,
                measure,
                depth + 1,
            )?;
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_layout_tree_tests.rs"]
mod tests;
