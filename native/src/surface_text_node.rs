//! Adapt declarative text nodes to native layout; tree-layout owners explicitly supply inherited styles.
use crate::{
    surface_length::Context,
    surface_scene::{Kind, Node},
    surface_text::{Align, Engine, Layout, Style, Whitespace},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

/// Do not rewrite nodes or parent styles; invalid allowlisted attributes follow mainline rules and retain inherited values.
pub fn resolve_style<'a>(node: &Node, mut inherited: Style<'a>, mut context: Context) -> Style<'a> {
    // em font sizes are relative to parent fonts, not potentially stale current-node sizes in caller caches.
    context.font = inherited.size;
    // font-size percentages also reference parent font sizes, not layout-container width.
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
    // line-height em/% uses the node's resolved font size; inherit parent line height as its computed value.
    context.font = inherited.size;
    context.axis = inherited.size;
    if node.style().get("lineHeight").and_then(|v| v.as_str()) == Some("normal") {
        // CSS shorthand normal must clear ancestors' explicit line spacing and let actual fonts supply baselines again.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_text_node_tests.rs"]
mod tests;
