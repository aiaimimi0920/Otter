//! Grid measurement wiring: allocate columns before measuring actual cell widths; the original eight-round text/image feedback remains the sole convergence gate.
use super::*;
use crate::{surface_container::Measured, surface_grid};
pub(super) struct Cache {
    // NativeMeasure's Styles borrow a fixed scene; use node addresses only within that owner without dereferencing or reusing across scenes.
    values: BTreeMap<(usize, [u32; 2]), Intrinsic>,
    misses: usize,
}
impl Cache {
    pub(super) fn for_scene(scene: &Scene) -> Option<Self> {
        let mut pending = vec![scene.root()];
        while let Some(node) = pending.pop() {
            if node.kind() == Kind::Grid {
                return Some(Self {
                    values: BTreeMap::new(),
                    misses: 0,
                });
            }
            pending.extend(node.children());
        }
        None
    }
    pub(super) fn get(&self, node: &Node, available: [f32; 2]) -> Option<Intrinsic> {
        self.values
            .get(&(node as *const Node as usize, available.map(f32::to_bits)))
            .copied()
    }
    pub(super) fn reserve(&mut self) -> bool {
        self.misses += 1;
        // Multi-constraint measurement of nested Grids must not grow exponentially; exceeding the budget fails the entire frame without partial geometry.
        self.misses <= crate::surface_scene::MAX_NODES * 4
    }
    pub(super) fn insert(&mut self, node: &Node, available: [f32; 2], value: Intrinsic) {
        self.values.insert(
            (node as *const Node as usize, available.map(f32::to_bits)),
            value,
        );
    }
    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.misses = 0;
    }
}
impl NativeMeasure<'_, '_> {
    pub(super) fn grid_intrinsic(
        &mut self,
        node: &Node,
        available: [f32; 2],
        own: [f32; 2],
        inner: [f32; 2],
        context: Context,
        depth: usize,
    ) -> Result<Intrinsic> {
        let children = node.visible_children().collect::<Vec<_>>();
        let mut measured = children
            .iter()
            .map(|child| {
                let value = self.intrinsic(child, inner, context, depth + 1)?;
                Ok(Measured {
                    id: child.id(),
                    size: value.size,
                    baseline: value.baseline,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let definite = length(node, "height", available[1], context).is_some();
        let initial = surface_grid::layout(
            node,
            &measured,
            own,
            context,
            &|n| self.styles.get(n).map(|s| s.size),
            available[0],
            definite,
        )
        .map_err(|_| invalid())?;
        for ((child, value), cell) in children.iter().zip(&mut measured).zip(initial.cells) {
            // Preserve each column's initial content contribution; shorter wrapped lines must not shrink auto tracks in reverse.
            let next = self.intrinsic(child, cell, context, depth + 1)?;
            value.size[1] = next.size[1];
            value.baseline = next.baseline;
        }
        let final_layout = surface_grid::layout(
            node,
            &measured,
            own,
            context,
            &|n| self.styles.get(n).map(|s| s.size),
            available[0],
            definite,
        )
        .map_err(|_| invalid())?;
        if !valid(final_layout.extent) {
            return Err(invalid());
        }
        let value = Intrinsic {
            size: final_layout.extent,
            baseline: None,
        };
        self.containers.insert(node.id().to_owned(), value);
        Ok(value)
    }
}
