//! 单个场景的文字样式预计算；绑定节点引用，避免同 ID 新场景复用过期结果。
use crate::{
    surface_length::{Context, Length, Unit},
    surface_scene::{Kind, Node, Scene, MAX_DEPTH, MAX_NODES},
    surface_text::{Engine, Layout, Style},
    surface_text_node::resolve_style,
};
use std::collections::BTreeMap;
use std::rc::Rc;
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
struct Entry<'scene, 'font> {
    node: &'scene Node,
    style: Style<'font>,
    family: Option<Rc<str>>,
}
pub struct Styles<'scene, 'font> {
    entries: BTreeMap<&'scene str, Entry<'scene, 'font>>,
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid Surface inherited text style")
}
fn validate(style: Style<'_>) -> Result<()> {
    if style.family.is_empty()
        || style.family.len() > 256
        || style.family.chars().any(char::is_control)
        || !style.size.is_finite()
        || !(0. ..=8192.).contains(&style.size)
        || !(100..=900).contains(&style.weight)
        || style
            .line_height
            .is_some_and(|v| !v.is_finite() || !(0. ..=32768.).contains(&v))
    {
        return Err(invalid());
    }
    Ok(())
}
impl<'scene, 'font> Styles<'scene, 'font> {
    pub fn build(
        scene: &'scene Scene,
        base: Style<'font>,
        context: Context,
        engine: &Engine,
    ) -> Result<Self> {
        validate(base)?;
        if Length::Scalar(0., Unit::Px).resolve(context).is_none() {
            return Err(invalid());
        }
        let mut entries = BTreeMap::new();
        let mut pending = vec![(scene.root(), base, None, 1)];
        while let Some((node, inherited, mut family, depth)) = pending.pop() {
            if depth > MAX_DEPTH || entries.len() >= MAX_NODES {
                return Err(invalid());
            }
            let style = if node.visible() {
                resolve_style(node, inherited, context)
            } else {
                inherited
            };
            validate(style)?;
            if node.visible() {
                if let Some(value) = node.style().get("fontFamily") {
                    family = Some(engine.resolve_family(value.as_str().ok_or_else(invalid)?)?);
                }
            }
            if entries
                .insert(
                    node.id(),
                    Entry {
                        node,
                        style,
                        family: family.clone(),
                    },
                )
                .is_some()
            {
                return Err(invalid());
            }
            if !node.visible() {
                continue;
            }
            pending.extend(
                node.visible_children()
                    .rev()
                    .map(|child| (child, style, family.clone(), depth + 1)),
            );
        }
        Ok(Self { entries })
    }
    pub fn get(&self, node: &Node) -> Option<Style<'_>> {
        let entry = self.entries.get(node.id())?;
        if !std::ptr::eq(entry.node, node) {
            return None;
        }
        let mut style = entry.style;
        if let Some(family) = &entry.family {
            style.family = family;
        }
        Some(style)
    }
    /// 样式已由全树预计算解析，不能再次走 layout_node 重新计算相对单位。
    pub fn layout_text(
        &self,
        engine: &Engine,
        node: &Node,
        width: f32,
        height: f32,
    ) -> Result<Layout> {
        if !matches!(node.kind(), Kind::Text | Kind::Button | Kind::Icon) {
            return Err(invalid());
        }
        let style = self.get(node).ok_or_else(invalid)?;
        let text = crate::surface_button::content(node);
        engine.layout(text, style, width, height)
    }
}
#[cfg(test)]
#[path = "surface_text_styles_tests.rs"]
mod tests;
