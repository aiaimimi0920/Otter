//! Surface border-box content insets; unify measurement and layout so borders occupy layout space rather than being only painted.
use crate::{
    surface_length::{Context, Length, Unit},
    surface_scene::Node,
};

/// Four-side insets in logical pixels; separate the origin from axis totals, never substituting one side for an entire axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}
impl From<f32> for Insets {
    fn from(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
}
impl Insets {
    pub fn start(self) -> [f32; 2] {
        [self.left, self.top]
    }
    pub fn total(self) -> [f32; 2] {
        [self.left + self.right, self.top + self.bottom]
    }
    pub fn with_border(self, border: impl Into<Insets>) -> Self {
        let border = border.into();
        Self {
            left: self.left + border.left,
            top: self.top + border.top,
            right: self.right + border.right,
            bottom: self.bottom + border.bottom,
        }
    }
}

pub fn border(node: &Node, context: Context) -> f32 {
    width(node, "borderWidth", context)
}
fn width(node: &Node, key: &str, context: Context) -> f32 {
    if !node.style().contains_key(key) {
        return 0.;
    }
    // The frontend enables solid whenever borderWidth is provided; invalid widths fall back to CSS medium (3px).
    // border-width accepts no percentages, auto, fr, or positioning expressions.
    match node.style_length(key) {
        Some(
            length @ (Length::Scalar(
                _,
                Unit::Px | Unit::Em | Unit::Rem | Unit::Vw | Unit::Vh | Unit::Cqw | Unit::Cqh,
            )
            | Length::ContainerMin(..)),
        ) => length.resolve(context).unwrap_or(3.),
        _ => 3.,
    }
}

/// Non-inherited side longhands override only their own directions; preserve existing declarative single-value and invalid-value fallback semantics.
pub fn borders(node: &Node, context: Context) -> Insets {
    let uniform = border(node, context);
    let side = |key| {
        if node.style().contains_key(key) {
            width(node, key, context)
        } else {
            uniform
        }
    };
    Insets {
        left: side("borderLeftWidth"),
        top: side("borderTopWidth"),
        right: side("borderRightWidth"),
        bottom: side("borderBottomWidth"),
    }
}

/// Preserve declarative single-value padding; independent longhands override their sides, with defaults from controls rather than rewriting the original declaration.
pub fn padding(node: &Node, width: f32, mut context: Context, default: [f32; 2]) -> Insets {
    context.axis = width;
    let uniform = node
        .layout_length("padding")
        .and_then(|v| v.resolve(context));
    let side = |key, axis| {
        node.layout_length(key)
            .and_then(|v| v.resolve(context))
            .or(uniform)
            .unwrap_or(default[axis])
    };
    Insets {
        left: side("paddingLeft", 0),
        top: side("paddingTop", 1),
        right: side("paddingRight", 0),
        bottom: side("paddingBottom", 1),
    }
}
pub fn inset(node: &Node, width: f32, context: Context) -> Insets {
    padding(node, width, context, [0.; 2]).with_border(borders(node, context))
}
pub fn inner(size: [f32; 2], inset: impl Into<Insets>) -> [f32; 2] {
    let total = inset.into().total();
    std::array::from_fn(|axis| (size[axis] - total[axis]).max(0.))
}
