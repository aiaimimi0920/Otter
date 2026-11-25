//! Bounded text-reflow height feedback; deliver the same scene snapshot only after geometry and text measurements stabilize.
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
    /// Consume preparation results and release layout-tree borrows; native text snapshots may remain independently in window state.
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
    /// Each rebuild starts from initial measurement; failures return no partial PreparedScene, allowing callers to retain the previous one.
    pub fn prepare(
        &mut self,
        scene: &'scene Scene,
        available: [f32; 2],
        context: Context,
    ) -> Result<PreparedScene<'scene>> {
        self.prepare_bounded(scene, available, context, MAX_PASSES)
    }
    pub(super) fn prepare_bounded(
        &mut self,
        scene: &'scene Scene,
        available: [f32; 2],
        context: Context,
        max_passes: usize,
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
        for pass in 1..=max_passes {
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
            // The last round may still converge, but failure does not calculate unused intrinsic dimensions for another round.
            if pass == max_passes {
                break;
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
            format!("Surface text/image reflow did not converge within {max_passes} passes"),
        ))
    }
}
