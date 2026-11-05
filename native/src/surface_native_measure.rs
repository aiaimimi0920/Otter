//! 原生场景测量 owner：样式、DirectWrite 与容器固有尺寸同属一个场景生命周期。
use crate::{
    surface_layout_tree::{Intrinsic, Measure},
    surface_length::Context,
    surface_scene::{Kind, Node, Scene, MAX_DEPTH},
    surface_text::{Engine, Style},
    surface_text_styles::Styles,
};
use std::collections::BTreeMap;
#[path = "surface_text_frame.rs"]
mod frame;
pub use frame::{TextFrame, TextItem};
#[path = "surface_grid_measure.rs"]
mod grid_measure;
#[path = "surface_image_measure.rs"]
mod image_measure;
#[path = "surface_image_placeholder.rs"]
pub(crate) mod placeholder;
#[path = "surface_reflow.rs"]
mod reflow;
pub use reflow::PreparedScene;
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
pub struct NativeMeasure<'scene, 'font> {
    context: Context,
    engine: Engine,
    styles: Styles<'scene, 'font>,
    containers: BTreeMap<String, Intrinsic>,
    reflow: BTreeMap<String, Intrinsic>,
    images: BTreeMap<String, [f32; 2]>,
    image_reflow: BTreeMap<String, Intrinsic>,
    grid_cache: Option<grid_measure::Cache>,
}
fn invalid() -> Error {
    Error::new(
        E_INVALIDARG,
        "Unsupported or invalid Surface native measurement",
    )
}
fn valid(size: [f32; 2]) -> bool {
    size.iter()
        .all(|v| v.is_finite() && (0. ..=32768.).contains(v))
}
fn length(node: &Node, key: &str, axis: f32, mut context: Context) -> Option<f32> {
    context.axis = axis;
    crate::surface_layout_value::length(node, key, context)
}
impl<'scene, 'font> NativeMeasure<'scene, 'font> {
    pub(crate) fn resolved_style(&self, node: &Node) -> Option<Style<'_>> {
        self.styles.get(node)
    }
    pub fn new(
        scene: &'scene Scene,
        base: Style<'font>,
        context: Context,
        available: [f32; 2],
    ) -> Result<Self> {
        Self::with_images(scene, base, context, available, BTreeMap::new())
    }
    /// 自然尺寸来自已完成解码的当前场景资源；此入口不授予网络/资源读权。
    pub fn with_images(
        scene: &'scene Scene,
        base: Style<'font>,
        context: Context,
        available: [f32; 2],
        images: BTreeMap<String, [u32; 2]>,
    ) -> Result<Self> {
        Self::with_image_sizes(
            scene,
            base,
            context,
            available,
            images
                .into_iter()
                .map(|(id, size)| (id, size.map(|v| v as f32)))
                .collect(),
        )
    }
    /// 布局尺寸不是整数像素尺寸；只接受当前图片 owner 提供的有限正值。
    pub(crate) fn with_image_sizes(
        scene: &'scene Scene,
        base: Style<'font>,
        context: Context,
        available: [f32; 2],
        images: BTreeMap<String, [f32; 2]>,
    ) -> Result<Self> {
        if !valid(available) || images.len() > crate::surface_scene::MAX_NODES {
            return Err(invalid());
        }
        let engine = Engine::new()?;
        let styles = Styles::build(scene, base, context, &engine)?;
        let mut owner = Self {
            context,
            engine,
            styles,
            containers: BTreeMap::new(),
            reflow: BTreeMap::new(),
            images,
            image_reflow: BTreeMap::new(),
            grid_cache: grid_measure::Cache::for_scene(scene),
        };
        owner.intrinsic(scene.root(), available, context, 1)?;
        Ok(owner)
    }
    fn text(&self, node: &Node, available: [f32; 2], mut context: Context) -> Result<Intrinsic> {
        context.font = self.styles.get(node).ok_or_else(invalid)?.size;
        let width = length(node, "width", available[0], context).unwrap_or(available[0]);
        let inset = crate::surface_button::insets(node, available[0], context);
        let layout = self.styles.layout_text(
            &self.engine,
            node,
            (width - inset.total()[0]).max(0.),
            32768.,
        )?;
        let content = layout.size();
        let size = std::array::from_fn(|axis| {
            (content[axis] + inset.total()[axis]).max(crate::surface_button::minimum(
                node,
                available[0],
                context,
                axis,
            ))
        });
        if !valid(size) {
            return Err(invalid());
        }
        let mut result = Intrinsic {
            size,
            baseline: Some(
                layout.baseline()
                    + inset.top
                    + if node.kind() == Kind::Button {
                        (size[1] - content[1] - inset.total()[1]).max(0.) / 2.
                    } else {
                        0.
                    },
            ),
        };
        // 保留初始宽度基准，避免把每轮断行后的文字宽度当成新的 flex 基础尺寸。
        if let Some(final_text) = self.reflow.get(node.id()) {
            result.size[1] = final_text.size[1].max(crate::surface_button::minimum(
                node,
                available[0],
                context,
                1,
            ));
            result.baseline = final_text.baseline;
        }
        Ok(result)
    }
    fn intrinsic(
        &mut self,
        node: &Node,
        available: [f32; 2],
        context: Context,
        depth: usize,
    ) -> Result<Intrinsic> {
        if let Some(cache) = &mut self.grid_cache {
            if let Some(value) = cache.get(node, available) {
                return Ok(value);
            }
            if !cache.reserve() {
                return Err(invalid());
            }
        }
        let value = self.intrinsic_uncached(node, available, context, depth)?;
        if let Some(cache) = &mut self.grid_cache {
            cache.insert(node, available, value);
        }
        Ok(value)
    }
    fn intrinsic_uncached(
        &mut self,
        node: &Node,
        available: [f32; 2],
        mut context: Context,
        depth: usize,
    ) -> Result<Intrinsic> {
        if depth > MAX_DEPTH || !valid(available) {
            return Err(invalid());
        }
        context.font = self.styles.get(node).ok_or_else(invalid)?.size;
        if !node.visible() {
            let value = Intrinsic {
                size: [0.; 2],
                baseline: None,
            };
            self.containers.insert(node.id().to_owned(), value);
            return Ok(value);
        }
        if matches!(node.kind(), Kind::Text | Kind::Button | Kind::Icon) {
            if node.kind() != Kind::Icon && !node.children().is_empty() {
                return Err(invalid());
            }
            return self.text(node, available, context);
        }
        if node.kind() == Kind::Image {
            return self.image_intrinsic(node, available, context);
        }
        if matches!(
            node.kind(),
            Kind::Input | Kind::Textarea | Kind::Number | Kind::Select | Kind::Slider
        ) {
            if !node.children().is_empty() {
                return Err(invalid());
            }
            if node.kind() == Kind::Select {
                crate::surface_select::Options::parse(node).map_err(|_| invalid())?;
            }
            let rows = if node.kind() == Kind::Textarea {
                node.props()
                    .get("rows")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(3.)
                    .clamp(1., 20.)
                    .trunc() as f32
            } else {
                1.
            };
            let value = Intrinsic {
                size: [
                    available[0],
                    rows * context.font * 1.4
                        + if node.kind() == Kind::Textarea {
                            crate::surface_textarea::insets(node, available[0], context).total()[1]
                        } else {
                            12.
                        },
                ],
                baseline: None,
            };
            if !valid(value.size) {
                return Err(invalid());
            }
            self.containers.insert(node.id().to_owned(), value);
            return Ok(value);
        }
        if matches!(
            node.kind(),
            Kind::Spacer | Kind::Divider | Kind::Progress | Kind::Switch
        ) {
            if !node.children().is_empty() {
                return Err(invalid());
            }
            // Spacer 固有内容为零；Divider 使用固定 1px 线高，二者均不创建文字布局。
            let value = Intrinsic {
                size: if node.kind() == Kind::Switch {
                    crate::surface_switch::SIZE
                } else if node.kind() == Kind::Progress {
                    [available[0], context.font]
                } else if node.kind() == Kind::Divider {
                    [available[0], 1.]
                } else {
                    crate::surface_box::inset(node, available[0], context).total()
                },
                baseline: None,
            };
            if !valid(value.size) {
                return Err(invalid());
            }
            self.containers.insert(node.id().to_owned(), value);
            return Ok(value);
        }
        if !matches!(
            node.kind(),
            Kind::View | Kind::Row | Kind::Column | Kind::Stack | Kind::Scroll | Kind::Grid
        ) {
            return Err(invalid());
        }
        let own = [
            length(node, "width", available[0], context).unwrap_or(available[0]),
            length(node, "height", available[1], context).unwrap_or(available[1]),
        ];
        if !valid(own) {
            return Err(invalid());
        }
        let padding = crate::surface_box::inset(node, available[0], context);
        let inner = crate::surface_box::inner(own, padding);
        if node.kind() == Kind::Grid {
            return self.grid_intrinsic(node, available, own, inner, context, depth);
        }
        let main = usize::from(node.kind() != Kind::Row);
        let cross = 1 - main;
        let gap = length(node, "gap", inner[main], context).unwrap_or(0.);
        let mut size = [0_f32; 2];
        let mut count = 0;
        for child in node.visible_children() {
            let absolute = child
                .layout()
                .get("position")
                .is_some_and(|v| v == "absolute");
            let child_available = if absolute {
                crate::surface_box::inner(own, crate::surface_box::borders(node, context))
            } else {
                inner
            };
            let value = self.intrinsic(child, child_available, context, depth + 1)?;
            if absolute {
                continue;
            }
            let mut child_context = context;
            child_context.font = self.styles.get(child).ok_or_else(invalid)?.size;
            let child_size = [
                length(child, "width", inner[0], child_context).unwrap_or(value.size[0]),
                length(child, "height", inner[1], child_context).unwrap_or(value.size[1]),
            ];
            if node.kind() == Kind::Stack {
                size[0] = size[0].max(child_size[0]);
                size[1] = size[1].max(child_size[1]);
            } else {
                size[main] += child_size[main];
                size[cross] = size[cross].max(child_size[cross]);
            }
            count += 1;
        }
        if node.kind() != Kind::Stack && count > 1 {
            size[main] += gap * (count - 1) as f32;
        }
        for (size, inset) in size.iter_mut().zip(padding.total()) {
            *size += inset;
        }
        if !valid(size) {
            return Err(invalid());
        }
        let value = Intrinsic {
            size,
            baseline: None,
        };
        self.containers.insert(node.id().to_owned(), value);
        Ok(value)
    }
}
impl Measure for NativeMeasure<'_, '_> {
    type Error = Error;
    fn font_size(&self, node: &Node) -> Option<f32> {
        self.styles.get(node).map(|style| style.size)
    }
    fn measure(&mut self, node: &Node, available: [f32; 2], context: Context) -> Result<Intrinsic> {
        if !valid(available) || self.styles.get(node).is_none() {
            return Err(invalid());
        }
        if matches!(node.kind(), Kind::Text | Kind::Button | Kind::Icon) {
            self.text(node, available, context)
        } else if node.kind() == Kind::Image {
            self.image_intrinsic(node, available, context)
        } else {
            if self.grid_cache.is_some() {
                self.intrinsic(node, available, context, 1)
            } else {
                self.containers.get(node.id()).copied().ok_or_else(invalid)
            }
        }
    }
}
#[cfg(test)]
#[path = "surface_native_measure_tests.rs"]
mod tests;
