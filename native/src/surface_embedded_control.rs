//! 无子HWND的只读控件绘制；复用既有值/字体/几何合同，不建立draft或输入权限。
use crate::{
    surface_box::Insets,
    surface_clip::Rect,
    surface_presentation::ControlFont,
    surface_scene::{Kind, Node},
    surface_text::{Align, Engine, Layout, Style, Whitespace},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::Direct2D::{Common::*, *},
    },
};
pub(crate) struct Marker {
    pub id: String,
    pub ink: crate::surface_button_visual::PaintColor,
    origin: [f32; 2],
    size: [f32; 2],
    clip: Rect,
    inset: Insets,
}
impl Marker {
    pub fn new(
        placed: &crate::surface_layout_tree::Placed<'_>,
        clip: Rect,
        context: crate::surface_length::Context,
        declared: Option<crate::surface_color::Color>,
    ) -> Self {
        let inset = if placed.node.kind() == Kind::Textarea {
            crate::surface_textarea::insets(placed.node, placed.containing_width, context)
        } else {
            crate::surface_box::inset(placed.node, placed.containing_width, context)
        };
        let border = crate::surface_box::borders(placed.node, context);
        let start = border.start();
        let inner = crate::surface_box::inner(placed.size, border);
        Self {
            id: placed.node.id().to_owned(),
            ink: crate::surface_button_visual::PaintColor::new(
                placed.node,
                "color",
                declared.unwrap_or(crate::surface_color::Color([
                    247. / 255.,
                    248. / 255.,
                    239. / 255.,
                    1.,
                ])),
            ),
            origin: placed.origin,
            size: placed.size,
            inset,
            clip: clip.intersect(Rect::from_box(
                [placed.origin[0] + start[0], placed.origin[1] + start[1]],
                inner,
            )),
        }
    }
}
// 布局时冻结的Insets包含canonical viewport/containing width，不按控件自身尺寸另算百分比。
pub(crate) fn prepare<'a, 'font>(
    scene: &crate::surface_scene::Scene,
    markers: impl Iterator<Item = &'a Marker>,
    font: impl Fn(&str) -> Option<&'font ControlFont>,
) -> Result<std::collections::BTreeMap<String, Paint>> {
    fn find<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
        if node.id() == id {
            return Some(node);
        }
        node.children().iter().find_map(|n| find(n, id))
    }
    let engine = Engine::new()?;
    markers
        .map(|m| {
            let node = find(scene.root(), &m.id).expect("paint owns scene node");
            Paint::new(node, m, font(&m.id), &engine, None).map(|p| (m.id.clone(), p))
        })
        .collect()
}
pub(crate) enum Paint {
    Text {
        layout: Layout,
        origin: [f32; 2],
        clip: Rect,
        background: Option<Rect>,
    },
    Slider {
        rect: Rect,
        fraction: f32,
        clip: Rect,
        disabled: bool,
    },
}
#[cfg(test)]
#[path = "surface_embedded_control_tests.rs"]
mod tests;
pub(crate) fn is_control(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Input | Kind::Textarea | Kind::Number | Kind::Select | Kind::Slider
    )
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid embedded Surface control")
}
impl Paint {
    pub(crate) fn text_preview(
        node: &Node,
        marker: &Marker,
        font: Option<&ControlFont>,
        value: Option<&str>,
    ) -> Result<Self> {
        if !matches!(
            node.kind(),
            Kind::Input | Kind::Textarea | Kind::Number | Kind::Select
        ) {
            return Err(invalid());
        }
        Self::new(node, marker, font, &Engine::new()?, value)
    }
    fn new(
        node: &Node,
        marker: &Marker,
        font: Option<&ControlFont>,
        engine: &Engine,
        preview: Option<&str>,
    ) -> Result<Self> {
        let Marker {
            origin,
            size,
            clip,
            inset,
            ..
        } = *marker;
        if node.kind() == Kind::Slider {
            let range = crate::surface_slider::Range::parse(node).map_err(|_| invalid())?;
            return Ok(Self::Slider {
                rect: Rect::from_box(origin, size),
                fraction: range.fraction(range.source()).map_err(|_| invalid())? as f32,
                clip,
                disabled: node.props().get("disabled") == Some(&serde_json::Value::Bool(true)),
            });
        }
        let font = font.ok_or_else(invalid)?;
        let text = if node.kind() == Kind::Select {
            let options = crate::surface_select::Options::parse(node).map_err(|_| invalid())?;
            preview.map(str::to_owned).unwrap_or_else(|| {
                options
                    .source_index()
                    .map(|i| options.items()[i].label())
                    .unwrap_or("")
                    .to_owned()
            })
        } else {
            let value = node.props().get("value");
            let value = match value {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Number(n)) if node.kind() == Kind::Number => n.to_string(),
                _ => String::new(),
            };
            let value = preview.unwrap_or(&value);
            if value.is_empty() {
                node.props()
                    .get("placeholder")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            } else {
                value.to_owned()
            }
        };
        let start = inset.start();
        let total = inset.total();
        let available = [(size[0] - total[0]).max(0.), (size[1] - total[1]).max(0.)];
        let layout = engine.layout(
            &text,
            Style {
                family: &font.family,
                size: font.size,
                weight: font.weight,
                whitespace: if node.kind() == Kind::Textarea {
                    Whitespace::PreWrap
                } else {
                    Whitespace::Nowrap
                },
                align: Align::Start,
                line_height: None,
            },
            available[0],
            available[1],
        )?;
        let y = if node.kind() == Kind::Textarea {
            start[1]
        } else {
            start[1] + (available[1] - layout.size()[1]).max(0.) / 2.
        };
        Ok(Self::Text {
            layout,
            origin: [origin[0] + start[0], origin[1] + y],
            clip,
            background: (!node.explicit_style("background")).then(|| Rect::from_box(origin, size)),
        })
    }
    pub fn draw(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) -> Result<()> {
        let clip = match self {
            Self::Text { clip, .. } | Self::Slider { clip, .. } => *clip,
        };
        if clip.empty() {
            return Ok(());
        }
        let color = unsafe { brush.GetColor() };
        unsafe {
            target.PushAxisAlignedClip(
                &D2D_RECT_F {
                    left: clip.left,
                    top: clip.top,
                    right: clip.right,
                    bottom: clip.bottom,
                },
                D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            );
        }
        let result = match self {
            Self::Text {
                layout,
                origin,
                background,
                ..
            } => {
                if let Some(r) = background {
                    unsafe {
                        brush.SetColor(&crate::surface_control_colors::TRACK);
                        target.FillRectangle(
                            &D2D_RECT_F {
                                left: r.left,
                                top: r.top,
                                right: r.right,
                                bottom: r.bottom,
                            },
                            brush,
                        );
                        brush.SetColor(&color);
                    }
                }
                layout.draw(target, brush, *origin)
            }
            Self::Slider {
                rect,
                fraction,
                disabled,
                ..
            } => {
                use crate::surface_control_colors as colors;
                let y = (rect.top + rect.bottom) / 2.;
                let left = rect.left + 5.;
                let right = (rect.right - 5.).max(left);
                let x = left + (right - left) * fraction;
                unsafe {
                    brush.SetColor(&colors::MUTED);
                    target.DrawLine(
                        D2D_POINT_2F { x: left, y },
                        D2D_POINT_2F { x: right, y },
                        brush,
                        2.,
                        None,
                    );
                    brush.SetColor(if *disabled {
                        &colors::MUTED
                    } else {
                        &colors::ACTIVE
                    });
                    target.FillEllipse(
                        &D2D1_ELLIPSE {
                            point: D2D_POINT_2F { x, y },
                            radiusX: 5.,
                            radiusY: 5.,
                        },
                        brush,
                    );
                }
                Ok(())
            }
        };
        unsafe {
            target.PopAxisAlignedClip();
            brush.SetColor(&color);
        }
        result
    }
}
