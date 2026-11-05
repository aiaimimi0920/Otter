//! 文字重排高度的有界反馈；只有几何和文字度量稳定后才交付同一场景快照。
use super::*;
use crate::surface_layout_tree::Tree;
use windows::Win32::Foundation::E_FAIL;
const MAX_PASSES: usize = 8;
pub struct PreparedScene<'a> {
    tree: Tree<'a>,
    text: TextFrame,
    passes: usize,
}
impl<'a> PreparedScene<'a> {
    pub fn tree(&self) -> &Tree<'a> {
        &self.tree
    }
    pub fn text(&self) -> &TextFrame {
        &self.text
    }
    /// 消费准备结果，释放借用布局树；原生文字快照可独立留在窗口状态中。
    pub fn into_text(self) -> TextFrame {
        self.text
    }
    pub fn passes(&self) -> usize {
        self.passes
    }
}
fn equivalent(a: &BTreeMap<String, Intrinsic>, b: &BTreeMap<String, Intrinsic>) -> bool {
    a.len() == b.len()
        && a.iter().all(|(id, a)| {
            b.get(id).is_some_and(|b| {
                (a.size[1] - b.size[1]).abs() <= 0.01
                    && match (a.baseline, b.baseline) {
                        (Some(a), Some(b)) => (a - b).abs() <= 0.01,
                        (None, None) => true,
                        _ => false,
                    }
            })
        })
}
impl<'scene> NativeMeasure<'scene, '_> {
    /// 每次重建从初始测量开始；失败不返回部分 PreparedScene，调用方可保留上一份。
    pub fn prepare(
        &mut self,
        scene: &'scene Scene,
        available: [f32; 2],
        context: Context,
    ) -> Result<PreparedScene<'scene>> {
        if self.styles.get(scene.root()).is_none() || !valid(available) {
            return Err(invalid());
        }
        self.reflow.clear();
        self.image_reflow.clear();
        self.context = context;
        self.containers.clear();
        if let Some(cache) = &mut self.grid_cache {
            cache.clear();
        }
        self.intrinsic(scene.root(), available, context, 1)?;
        for pass in 1..=MAX_PASSES {
            let tree = Tree::build(scene, available, context, self)
                .map_err(|_| Error::new(E_FAIL, "Surface tree layout failed"))?;
            let text = self.text_frame(&tree)?;
            let images = self.image_feedback(&tree, context)?;
            let next = text
                .items()
                .iter()
                .map(|item| {
                    (
                        item.node_id().to_owned(),
                        Intrinsic {
                            size: std::array::from_fn(|axis| {
                                item.layout().size()[axis] + item.inset.total()[axis]
                            }),
                            baseline: Some(item.baseline),
                        },
                    )
                })
                .collect();
            if equivalent(&self.reflow, &next)
                && image_measure::equivalent(&self.image_reflow, &images)
            {
                return Ok(PreparedScene {
                    tree,
                    text,
                    passes: pass,
                });
            }
            self.reflow = next;
            self.image_reflow = images;
            self.containers.clear();
            if let Some(cache) = &mut self.grid_cache {
                cache.clear();
            }
            self.intrinsic(scene.root(), available, context, 1)?;
        }
        Err(Error::new(
            E_FAIL,
            "Surface text/image reflow did not converge within 8 passes",
        ))
    }
}
