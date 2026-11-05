//! 将声明式 text 节点适配到原生布局；继承样式由树布局 owner 显式传入。
use crate::{
    surface_length::Context,
    surface_scene::{Kind, Node},
    surface_text::{Align, Engine, Layout, Style, Whitespace},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

/// 不改写节点或父样式；非法白名单属性遵循主干规则，保留继承值。
pub fn resolve_style<'a>(node: &Node, mut inherited: Style<'a>, mut context: Context) -> Style<'a> {
    // em 字号相对于父字体，而不是调用方缓存中可能过期的当前节点字号。
    context.font = inherited.size;
    // font-size 百分比同样相对父字号，不是布局容器的宽度。
    context.axis = inherited.size;
    if let Some(size) = node
        .style_length("fontSize")
        .and_then(|value| value.resolve(context))
    {
        inherited.size = size;
    }
    if let Some(weight) = node
        .style()
        .get("fontWeight")
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
    {
        inherited.weight = weight.clamp(100., 900.) as u16;
    }
    inherited.whitespace = match node.style().get("whiteSpace").and_then(|v| v.as_str()) {
        Some("normal") => Whitespace::Normal,
        Some("nowrap") => Whitespace::Nowrap,
        Some("pre") => Whitespace::Pre,
        Some("pre-wrap") => Whitespace::PreWrap,
        _ => inherited.whitespace,
    };
    inherited.align = match node.style().get("textAlign").and_then(|v| v.as_str()) {
        Some("left") => Align::Start,
        Some("center") => Align::Center,
        Some("right") => Align::End,
        _ => inherited.align,
    };
    // line-height 的 em/% 使用节点已解析字号；父行高按已计算值继承。
    context.font = inherited.size;
    context.axis = inherited.size;
    if node.style().get("lineHeight").and_then(|v| v.as_str()) == Some("normal") {
        // CSS shorthand 的 normal 必须清除祖先显式行距，由实际字体重新提供基线。
        inherited.line_height = None;
    } else if let Some(height) = node
        .style_length("lineHeight")
        .and_then(|v| v.resolve(context))
    {
        inherited.line_height = Some(height);
    }
    inherited
}
impl Engine {
    pub fn layout_node(
        &self,
        node: &Node,
        inherited: Style<'_>,
        context: Context,
        width: f32,
        height: f32,
    ) -> Result<Layout> {
        if node.kind() != Kind::Text {
            return Err(Error::new(E_INVALIDARG, "Expected a Surface text node"));
        }
        let text = node
            .props()
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let mut style = resolve_style(node, inherited, context);
        let family = node
            .style()
            .get("fontFamily")
            .map(|value| {
                self.resolve_family(
                    value
                        .as_str()
                        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid Surface font family"))?,
                )
            })
            .transpose()?;
        if let Some(family) = &family {
            style.family = family;
        }
        self.layout(text, style, width, height)
    }
}
#[cfg(test)]
#[path = "surface_text_node_tests.rs"]
mod tests;
