//! 无当前像素的内联 Image 使用独立替代内容；不是伪造的自然图片尺寸或失败事件。
use super::*;
pub(crate) const ICON: f32 = 16.;
pub(crate) const TEXT_OFFSET: f32 = 20.;
pub(crate) fn alt(node: &Node) -> &str {
    node.props()
        .get("alt")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
}
pub(crate) fn layout(
    engine: &Engine,
    node: &Node,
    mut style: Style<'_>,
    width: f32,
    height: f32,
) -> Result<crate::surface_text::Layout> {
    // 只裁短绘制副本，原始 alt/UIA 名称不变；不允许超长替代文本扩大原生布局预算。
    let text: String = alt(node)
        .chars()
        .filter(|ch| *ch != '\0')
        .take(4096)
        .collect();
    style.whitespace = crate::surface_text::Whitespace::Nowrap;
    style.align = crate::surface_text::Align::Start;
    style.size = style.size.max(0.01);
    engine.layout(&text, style, width, height)
}
impl NativeMeasure<'_, '_> {
    pub(super) fn image_placeholder_intrinsic(
        &self,
        node: &Node,
        available: [f32; 2],
        mut context: Context,
    ) -> Result<Intrinsic> {
        let style = self.styles.get(node).ok_or_else(invalid)?;
        context.font = style.size;
        let inset = crate::surface_box::inset(node, available[0], context).total();
        let content = if alt(node).is_empty() {
            [ICON; 2]
        } else {
            let text = layout(&self.engine, node, style, 32768., 32768.)?.size();
            [(text[0] + TEXT_OFFSET).min(32768.), text[1].max(ICON)]
        };
        let size = std::array::from_fn(|axis| {
            length(node, ["width", "height"][axis], available[axis], context)
                .unwrap_or((content[axis] + inset[axis]).min(32768.))
                .max(inset[axis])
        });
        if !valid(size) {
            return Err(invalid());
        }
        Ok(Intrinsic {
            size,
            baseline: None,
        })
    }
}
