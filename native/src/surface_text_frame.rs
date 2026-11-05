//! 最终文字矩形的绘制快照；按已分配宽度重排，保留实际布局及溢出，不回写父几何。
use super::*;
use crate::{surface_layout_tree::Tree, surface_text::Layout};
use windows::Win32::Graphics::Direct2D::{
    Common::D2D_RECT_F, ID2D1RenderTarget, ID2D1SolidColorBrush, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
};
#[derive(Clone)]
pub struct TextItem {
    pub(super) inset: crate::surface_box::Insets,
    pub(super) baseline: f32,
    node_id: String,
    origin: [f32; 2],
    allocated: [f32; 2],
    layout: Layout,
    overflow: [f32; 2],
    clip: crate::surface_clip::Rect,
    selectable: bool,
}
impl TextItem {
    pub fn selectable(&self) -> bool {
        self.selectable
    }
    /// 开始选择必须位于当前可见文字内；被裁剪文字不能抢占其他控件的指针。
    pub fn selection_start(&self, point: [f32; 2]) -> Result<Option<u32>> {
        if !self.selectable
            || self.clip.empty()
            || point[0] < self.clip.left
            || point[0] >= self.clip.right
            || point[1] < self.clip.top
            || point[1] >= self.clip.bottom
        {
            return Ok(None);
        }
        let hit = self
            .layout
            .hit_text([point[0] - self.origin[0], point[1] - self.origin[1]])?;
        Ok(hit.inside.then_some(hit.position))
    }
    /// 已捕获的选择允许拖出内容盒，复用 DirectWrite 最近文字边沿。
    pub fn selection_position(&self, point: [f32; 2]) -> Result<u32> {
        if !self.selectable {
            return Err(invalid());
        }
        Ok(self
            .layout
            .hit_text([point[0] - self.origin[0], point[1] - self.origin[1]])?
            .position)
    }
    pub(crate) fn selection_caret(&self, point: [f32; 2]) -> Result<[f32; 2]> {
        if !self.selectable {
            return Err(invalid());
        }
        self.layout
            .point_caret([point[0] - self.origin[0], point[1] - self.origin[1]])
    }
    pub fn selection_rects(
        &self,
        range: std::ops::Range<u32>,
    ) -> Result<Vec<crate::surface_clip::Rect>> {
        if !self.selectable {
            return Err(invalid());
        }
        Ok(self
            .layout
            .selection_rects(range)?
            .into_iter()
            .map(|rect| {
                crate::surface_clip::Rect {
                    left: rect.left + self.origin[0],
                    top: rect.top + self.origin[1],
                    right: rect.right + self.origin[0],
                    bottom: rect.bottom + self.origin[1],
                }
                .intersect(self.clip)
            })
            .filter(|rect| !rect.empty())
            .collect())
    }
    pub fn clip(&self) -> crate::surface_clip::Rect {
        self.clip
    }
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
    pub fn origin(&self) -> [f32; 2] {
        self.origin
    }
    pub fn allocated(&self) -> [f32; 2] {
        self.allocated
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
    }
    pub fn overflow(&self) -> [f32; 2] {
        self.overflow
    }
}
#[derive(Clone)]
pub struct TextFrame {
    items: Vec<TextItem>,
}
impl TextFrame {
    /// 平移已测量的文字布局并重算裁剪；COM 布局引用共享，不重新测量字形。
    pub(crate) fn reposition(
        &mut self,
        tree: &Tree<'_>,
        border: impl Fn(&Node) -> crate::surface_box::Insets,
    ) {
        let clips = crate::surface_clip::ancestors_with_border(tree, border);
        let placed: BTreeMap<_, _> = tree
            .nodes()
            .iter()
            .zip(clips)
            .map(|(p, c)| (p.node.id(), (p, c)))
            .collect();
        for item in &mut self.items {
            if let Some((p, clip)) = placed.get(item.node_id.as_str()) {
                item.origin = std::array::from_fn(|a| p.origin[a] + item.inset.start()[a]);
                item.clip = clip.intersect(crate::surface_clip::Rect::from_box(
                    item.origin,
                    item.allocated,
                ));
                if matches!(p.node.kind(), Kind::Button | Kind::Icon) {
                    item.origin[1] += (item.allocated[1] - item.layout.size()[1]).max(0.) / 2.;
                }
                if p.node.kind() == Kind::Icon {
                    item.origin[0] +=
                        (item.allocated[0] - item.layout.allocated_width()).max(0.) / 2.;
                }
            }
        }
    }
    pub fn items(&self) -> &[TextItem] {
        &self.items
    }
    /// 已包含祖先裁剪；调用方负责 BeginDraw/EndDraw、颜色刷和滚动变换。
    pub fn draw(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) -> Result<()> {
        for index in 0..self.items.len() {
            self.draw_item(index, target, brush)?;
        }
        Ok(())
    }
    pub(crate) fn draw_item(
        &self,
        index: usize,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        let item = self.items.get(index).ok_or_else(invalid)?;
        if item.clip.empty() {
            return Ok(());
        }
        let rect = D2D_RECT_F {
            left: item.clip.left,
            top: item.clip.top,
            right: item.clip.right,
            bottom: item.clip.bottom,
        };
        unsafe {
            target.PushAxisAlignedClip(&rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }
        let result = item.layout.draw(target, brush, item.origin);
        // 即使绘制入口拒绝参数，也必须恢复目标的裁剪栈。
        unsafe {
            target.PopAxisAlignedClip();
        }
        result
    }
}
impl NativeMeasure<'_, '_> {
    pub fn text_frame(&self, tree: &Tree<'_>) -> Result<TextFrame> {
        let mut items = Vec::new();
        let clips = crate::surface_clip::ancestors_with_border(tree, |node| {
            crate::surface_box::borders(
                node,
                Context {
                    font: self
                        .styles
                        .get(node)
                        .map_or(self.context.font, |style| style.size),
                    ..self.context
                },
            )
        });
        for (placed, ancestor) in tree.nodes().iter().zip(clips) {
            let style = self.styles.get(placed.node).ok_or_else(invalid)?;
            let placeholder = placed.node.image_placeholder();
            if !matches!(placed.node.kind(), Kind::Text | Kind::Button | Kind::Icon) && !placeholder
            {
                continue;
            }
            if placeholder && super::placeholder::alt(placed.node).is_empty() {
                continue;
            }
            if !valid(placed.size) {
                return Err(invalid());
            }
            let mut inset = crate::surface_button::insets(
                placed.node,
                placed.containing_width,
                Context {
                    font: style.size,
                    ..self.context
                },
            );
            let mut allocated = crate::surface_box::inner(placed.size, inset);
            if placeholder {
                inset.left += super::placeholder::TEXT_OFFSET;
                allocated[0] = (allocated[0] - super::placeholder::TEXT_OFFSET).max(0.);
            }
            let mut origin = std::array::from_fn(|axis| placed.origin[axis] + inset.start()[axis]);
            let clip = ancestor.intersect(crate::surface_clip::Rect::from_box(origin, allocated));
            // 零面积内容盒没有可见像素，不创建无意义的 native 文本布局。
            if allocated.contains(&0.) {
                continue;
            }
            let text = crate::surface_button::content(placed.node);
            // 不重新读取显式 width：flex 收缩后的最终宽度才是重排依据。
            let mut layout = if placeholder {
                super::placeholder::layout(
                    &self.engine,
                    placed.node,
                    style,
                    allocated[0],
                    allocated[1],
                )?
            } else {
                self.engine
                    .layout_final(placed.node, text, style, allocated[0], allocated[1])?
            };
            if placed.node.kind() == Kind::Icon {
                // 网格居中的是内容盒，文字各行仍继承 textAlign；收窄后保留父级对齐。
                let width = layout.size()[0].ceil().min(allocated[0]);
                layout = self.engine.layout(text, style, width, allocated[1])?;
                origin[0] += (allocated[0] - width).max(0.) / 2.;
            }
            let size = layout.size();
            if matches!(placed.node.kind(), Kind::Button | Kind::Icon) {
                origin[1] += (allocated[1] - size[1]).max(0.) / 2.;
            }
            let baseline = layout.baseline() + origin[1] - placed.origin[1];
            let overflow = [
                (size[0] - allocated[0]).max(0.),
                (size[1] - allocated[1]).max(0.),
            ];
            items.push(TextItem {
                inset,
                baseline,
                node_id: placed.node.id().to_owned(),
                origin,
                allocated,
                layout,
                overflow,
                clip,
                selectable: placed.node.kind() == Kind::Text
                    && placed.node.props().get("selectable")
                        == Some(&serde_json::Value::Bool(true)),
            });
        }
        Ok(TextFrame { items })
    }
}
