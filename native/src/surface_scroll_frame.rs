//! 纯滚动呈现缓存：持有未平移几何和共享 DirectWrite 布局，绝不跨场景/重排复用。
use super::*;
use crate::surface_layout_tree::{Snapshot, Tree};
use std::collections::BTreeMap;
/// 原生字段使用与测量相同的已解析字体；不在滚动路径重新解析继承或相对单位。
#[derive(Clone)]
pub struct ControlFont {
    pub family: String,
    pub size: f32,
    pub weight: u16,
}
pub(super) struct Frame {
    pub text: TextFrame,
    pub geometry: Geometry,
    pub paint: PaintFrame,
    pub hits: crate::surface_hit_test::Hits,
}
pub(super) struct Cache {
    pub(super) identity: [u8; 32],
    layout: Snapshot,
    text: TextFrame,
    fonts: BTreeMap<String, f32>,
    control_fonts: BTreeMap<String, ControlFont>,
    textarea_boxes: BTreeMap<String, crate::surface_textarea::BoxStyle>,
    context: Context,
}
impl Cache {
    pub fn new<'font>(
        tree: &Tree<'_>,
        text: &TextFrame,
        context: Context,
        font: impl Fn(&crate::surface_scene::Node) -> Option<Style<'font>>,
    ) -> Self {
        Self {
            identity: layout_identity(tree),
            textarea_boxes: tree
                .nodes()
                .iter()
                .filter(|p| p.node.kind() == crate::surface_scene::Kind::Textarea)
                .map(|p| {
                    (
                        p.node.id().to_owned(),
                        crate::surface_textarea::project(
                            p,
                            Context {
                                font: font(p.node).map(|s| s.size).unwrap_or(context.font),
                                ..context
                            },
                        ),
                    )
                })
                .collect(),
            layout: Snapshot::new(tree),
            text: text.clone(),
            context,
            fonts: tree
                .nodes()
                .iter()
                .map(|p| {
                    (
                        p.node.id().to_owned(),
                        font(p.node).map(|s| s.size).unwrap_or(context.font),
                    )
                })
                .collect(),
            control_fonts: tree
                .nodes()
                .iter()
                .filter(|p| {
                    matches!(
                        p.node.kind(),
                        crate::surface_scene::Kind::Select
                            | crate::surface_scene::Kind::Input
                            | crate::surface_scene::Kind::Textarea
                            | crate::surface_scene::Kind::Number
                    )
                })
                .filter_map(|p| {
                    font(p.node).map(|s| {
                        (
                            p.node.id().to_owned(),
                            ControlFont {
                                family: s.family.to_owned(),
                                size: s.size,
                                weight: s.weight,
                            },
                        )
                    })
                })
                .collect(),
        }
    }
    pub fn control_font(&self, id: &str) -> Option<&ControlFont> {
        self.control_fonts.get(id)
    }
    pub fn textarea_box(&self, id: &str) -> Option<crate::surface_textarea::BoxStyle> {
        self.textarea_boxes.get(id).copied()
    }
    pub fn project_with_images(
        &self,
        scene: &Scene,
        scroll: &mut crate::surface_scroll::Scroll,
        images: &Images,
    ) -> Result<Frame> {
        let mut tree = self.layout.restore(scene).ok_or_else(|| {
            windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Surface scroll layout identity changed",
            )
        })?;
        let font = |node: &crate::surface_scene::Node| self.fonts.get(node.id()).copied();
        let border = |node: &crate::surface_scene::Node| {
            crate::surface_box::borders(
                node,
                Context {
                    font: font(node).unwrap_or(self.context.font),
                    ..self.context
                },
            )
        };
        scroll.apply(&mut tree, border);
        let mut text = self.text.clone();
        text.reposition(&tree, border);
        let geometry = Geometry::with_border(&tree, border);
        let paint = PaintFrame::build_with_images(&tree, &text, self.context, font, |node| {
            images.get(node.id()).cloned()
        });
        let hits = crate::surface_hit_test::Hits::build(&tree, self.context, font);
        Ok(Frame {
            text,
            geometry,
            paint,
            hits,
        })
    }
}

/// 只描述未滚动布局/样式，不包含权威值；本地滚动不能伪装成布局换代，也不能掩盖真换代。
fn layout_identity(tree: &Tree<'_>) -> [u8; 32] {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    for value in tree.viewport() {
        hash.update(&value.to_bits().to_le_bytes());
    }
    for p in tree.nodes() {
        let bytes = serde_json::to_vec(&(
            p.node.id(),
            p.node.kind() as u8,
            p.parent,
            p.origin,
            p.size,
            p.containing_width,
            p.grid_cell,
            p.node.layout(),
            p.node.style(),
        ))
        .expect("Surface layout JSON identity");
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    hash.finish().as_ref().try_into().expect("SHA256 length")
}
