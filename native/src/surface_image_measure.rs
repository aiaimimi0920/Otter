//! 解码自然尺寸与 flex 交叉轴反馈；不接触来源、网络、像素或 D2D。
use super::*;
impl NativeMeasure<'_, '_> {
    fn natural(&self, node: &Node) -> Result<[f32; 2]> {
        if let Some(size) = node.canvas_size() {
            return Ok(size.map(|v| v as f32));
        }
        let size = self.images.get(node.id()).ok_or_else(invalid)?;
        if size
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0 || !(32768. / *v).is_finite() || *v > 16384.0)
            || crate::viewport::checked_image_bytes(size[0].ceil() as u32, size[1].ceil() as u32)
                .is_none_or(|bytes| bytes > 64 * 1024 * 1024)
        {
            return Err(invalid());
        }
        Ok(*size)
    }
    pub(super) fn image_intrinsic(
        &self,
        node: &Node,
        available: [f32; 2],
        mut context: Context,
    ) -> Result<Intrinsic> {
        if node.image_placeholder() {
            return self.image_placeholder_intrinsic(node, available, context);
        }
        let natural = self.natural(node)?;
        context.font = self.styles.get(node).ok_or_else(invalid)?.size;
        let inset = crate::surface_box::inset(node, available[0], context).total();
        if !valid(inset) {
            return Err(invalid());
        }
        let width = length(node, "width", available[0], context);
        let height = length(node, "height", available[1], context);
        if node.canvas_size().is_some() && (natural[0] == 0. || natural[1] == 0.) {
            return Ok(Intrinsic {
                size: [
                    width.unwrap_or(natural[0] + inset[0]).max(inset[0]),
                    height.unwrap_or(natural[1] + inset[1]).max(inset[1]),
                ],
                baseline: None,
            });
        }
        let mut size = match (width, height) {
            (Some(w), Some(h)) => [w.max(inset[0]), h.max(inset[1])],
            (Some(w), None) => [
                w.max(inset[0]),
                (w - inset[0]).max(0.) * natural[1] / natural[0] + inset[1],
            ],
            (None, Some(h)) => [
                (h - inset[1]).max(0.) * natural[0] / natural[1] + inset[0],
                h.max(inset[1]),
            ],
            (None, None) => std::array::from_fn(|a| natural[a] + inset[a]),
        };
        if let Some(final_image) = self.image_reflow.get(node.id()) {
            if width.is_none() {
                size[0] = final_image.size[0];
            }
            if height.is_none() {
                size[1] = final_image.size[1];
            }
        }
        // 极端长宽比的 auto 基础尺寸只需覆盖布局引擎可表达范围；最终 max 仍由布局约束。
        for (axis, explicit) in [width, height].into_iter().enumerate() {
            if explicit.is_none() {
                size[axis] = size[axis].min(32768.);
            }
        }
        if !valid(size) {
            return Err(invalid());
        }
        Ok(Intrinsic {
            size,
            baseline: None,
        })
    }
    pub(super) fn image_feedback(
        &self,
        tree: &crate::surface_layout_tree::Tree<'_>,
        context: Context,
    ) -> Result<BTreeMap<String, Intrinsic>> {
        let mut next = BTreeMap::new();
        for placed in tree.nodes().iter().filter(|p| p.node.kind() == Kind::Image) {
            let node = placed.node;
            if node.canvas_size().is_some_and(|size| size.contains(&0)) {
                continue;
            }
            // 替代内容无自然宽高比，不参与成功图片的交叉轴反馈。
            if node.image_placeholder() {
                continue;
            }
            let natural = self.natural(node)?;
            let local = Context {
                font: self.styles.get(node).ok_or_else(invalid)?.size,
                ..context
            };
            let parent = placed.parent.map(|index| &tree.nodes()[index]);
            let available = placed.grid_cell.unwrap_or_else(|| {
                parent.map_or(tree.viewport(), |p| {
                    let parent_context = Context {
                        font: self
                            .styles
                            .get(p.node)
                            .map_or(local.font, |style| style.size),
                        ..context
                    };
                    let inset = if node
                        .layout()
                        .get("position")
                        .is_some_and(|v| v == "absolute")
                    {
                        crate::surface_box::borders(p.node, parent_context)
                    } else {
                        crate::surface_box::inset(p.node, p.containing_width, parent_context)
                    };
                    crate::surface_box::inner(p.size, inset)
                })
            });
            let mut size = self.image_intrinsic(node, available, local)?.size;
            // Grid 不走父 Row/Column 二分；auto 高度随已分配列宽，显式高的 auto 宽反之。
            let main = if placed.grid_cell.is_some() {
                usize::from(length(node, "height", available[1], local).is_none())
            } else {
                usize::from(!parent.is_some_and(|p| p.node.kind() == Kind::Row))
            };
            let cross = 1 - main;
            let key = if main == 0 { "width" } else { "height" };
            if length(node, key, available[main], local).is_none() {
                let inset = crate::surface_box::inset(node, placed.containing_width, local).total();
                size[main] = ((placed.size[cross] - inset[cross]).max(0.) * natural[main]
                    / natural[cross]
                    + inset[main])
                    .min(32768.);
            }
            if !valid(size) {
                return Err(invalid());
            }
            next.insert(
                node.id().to_owned(),
                Intrinsic {
                    size,
                    baseline: None,
                },
            );
        }
        Ok(next)
    }
}
pub(super) fn equivalent(a: &BTreeMap<String, Intrinsic>, b: &BTreeMap<String, Intrinsic>) -> bool {
    a.len() == b.len()
        && a.iter().all(|(id, a)| {
            b.get(id)
                .is_some_and(|b| (0..2).all(|axis| (a.size[axis] - b.size[axis]).abs() <= 0.01))
        })
}
