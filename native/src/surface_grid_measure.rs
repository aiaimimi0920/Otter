//! Grid 的测量接线；先分列再按实际 cell 宽度度量，原八轮文字/图片反馈仍是唯一收敛门。
use super::*;
use crate::{surface_container::Measured, surface_grid};
pub(super) struct Cache {
    values: BTreeMap<(String, [u32; 2]), Intrinsic>,
    misses: usize,
}
impl Cache {
    pub(super) fn for_scene(scene: &Scene) -> Option<Self> {
        let mut pending = vec![scene.root()];
        let mut grid = false;
        while let Some(node) = pending.pop() {
            grid |= node.kind() == Kind::Grid;
            pending.extend(node.children());
        }
        grid.then(|| Self {
            values: BTreeMap::new(),
            misses: 0,
        })
    }
    pub(super) fn get(&self, node: &Node, available: [f32; 2]) -> Option<Intrinsic> {
        self.values
            .get(&(node.id().to_owned(), available.map(f32::to_bits)))
            .copied()
    }
    pub(super) fn reserve(&mut self) -> bool {
        self.misses += 1;
        // 嵌套 Grid 的多约束测量不能指数增长；超预算整帧失败，不返回部分几何。
        self.misses <= crate::surface_scene::MAX_NODES * 4
    }
    pub(super) fn insert(&mut self, node: &Node, available: [f32; 2], value: Intrinsic) {
        self.values
            .insert((node.id().to_owned(), available.map(f32::to_bits)), value);
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
            // 保留列的初始内容贡献，不把断行后的短行反过来收窄 auto 轨道。
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
