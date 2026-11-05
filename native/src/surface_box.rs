//! Surface border-box 内容内缩；统一测量和排版，避免边框只绘制却不占布局空间。
use crate::{
    surface_length::{Context, Length, Unit},
    surface_scene::Node,
};

/// 逻辑像素中的四边内缩；起点与两轴总量分离，禁止用某一边代替整轴。
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

pub(crate) fn border(node: &Node, context: Context) -> f32 {
    width(node, "borderWidth", context)
}
fn width(node: &Node, key: &str, context: Context) -> f32 {
    if !node.style().contains_key(key) {
        return 0.;
    }
    // 前端只要提供 borderWidth 就启用 solid；无效宽度回到 CSS medium（3px）。
    // border-width 不接受百分比、auto、fr 或定位表达式。
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

/// 非继承的四边 longhand 只覆盖各自方向；保留既有声明式单值与非法值回退语义。
pub(crate) fn borders(node: &Node, context: Context) -> Insets {
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

/// 保留声明式单值 padding；独立 longhand 覆盖对应边，缺省来自控件而非修改原声明。
pub(crate) fn padding(node: &Node, width: f32, mut context: Context, default: [f32; 2]) -> Insets {
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
pub(crate) fn inset(node: &Node, width: f32, context: Context) -> Insets {
    padding(node, width, context, [0.; 2]).with_border(borders(node, context))
}
pub(crate) fn inner(size: [f32; 2], inset: impl Into<Insets>) -> [f32; 2] {
    let total = inset.into().total();
    std::array::from_fn(|axis| (size[axis] - total[axis]).max(0.))
}
