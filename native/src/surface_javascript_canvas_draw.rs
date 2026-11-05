//! 实际 D2D/DirectWrite 绘制；调用方持有离屏 bitmap 并成对 BeginDraw/EndDraw。
use super::commands::{self, Draw, State};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};
fn transform(target: &ID2D1RenderTarget, value: [f32; 6]) {
    unsafe {
        let mut matrix = Default::default();
        target.GetTransform(&mut matrix);
        [
            matrix.M11, matrix.M12, matrix.M21, matrix.M22, matrix.M31, matrix.M32,
        ] = value;
        target.SetTransform(&matrix);
    }
}
fn rect(value: [f32; 4]) -> D2D_RECT_F {
    let [x, y, w, h] = value;
    D2D_RECT_F {
        left: x.min(x + w),
        top: y.min(y + h),
        right: x.max(x + w),
        bottom: y.max(y + h),
    }
}
fn brush(target: &ID2D1RenderTarget, state: &State) -> Result<ID2D1SolidColorBrush> {
    let [r, g, b, a] =
        crate::surface_color::Color::parse(&state.color, crate::surface_color::Color::BLACK)
            .unwrap()
            .0;
    unsafe {
        target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r,
                g,
                b,
                a: a * state.alpha,
            },
            None,
        )
    }
}
fn stroke(factory: &ID2D1Factory, state: &State) -> Result<ID2D1StrokeStyle> {
    let cap = match state.cap.as_str() {
        "round" => D2D1_CAP_STYLE_ROUND,
        "square" => D2D1_CAP_STYLE_SQUARE,
        _ => D2D1_CAP_STYLE_FLAT,
    };
    let join = match state.join.as_str() {
        "round" => D2D1_LINE_JOIN_ROUND,
        "bevel" => D2D1_LINE_JOIN_BEVEL,
        _ => D2D1_LINE_JOIN_MITER,
    };
    let dashes: Vec<f32> = state.dash.iter().map(|v| v / state.width).collect();
    let custom = dashes.iter().any(|v| *v > 0.);
    unsafe {
        factory.CreateStrokeStyle(
            &D2D1_STROKE_STYLE_PROPERTIES {
                startCap: cap,
                endCap: cap,
                dashCap: cap,
                lineJoin: join,
                miterLimit: state.miter,
                dashStyle: if custom {
                    D2D1_DASH_STYLE_CUSTOM
                } else {
                    D2D1_DASH_STYLE_SOLID
                },
                dashOffset: state.dash_offset / state.width,
            },
            if custom { Some(&dashes) } else { None },
        )
    }
}
pub(super) fn draw(
    target: &ID2D1RenderTarget,
    factory: &ID2D1Factory,
    text_engine: &crate::surface_text::Engine,
    images: &super::images::Bitmaps,
    command: &Draw,
) -> Result<()> {
    unsafe {
        match command {
            Draw::Image { .. } => {
                super::images::draw(target, images, command)?;
            }
            Draw::Clear {
                rect: value,
                transform: matrix,
            } => {
                if value[2] == 0. || value[3] == 0. {
                    return Ok(());
                }
                transform(target, *matrix);
                target.PushAxisAlignedClip(&rect(*value), D2D1_ANTIALIAS_MODE_ALIASED);
                target.Clear(Some(&D2D1_COLOR_F::default()));
                target.PopAxisAlignedClip();
            }
            Draw::Rect {
                rect: value,
                state,
                stroke: outlined,
            } => {
                transform(target, state.transform);
                let brush = brush(target, state)?;
                if *outlined {
                    target.DrawRectangle(
                        &rect(*value),
                        &brush,
                        state.width,
                        &stroke(factory, state)?,
                    );
                } else if value[2] != 0. && value[3] != 0. {
                    target.FillRectangle(&rect(*value), &brush);
                }
            }
            Draw::Path {
                path,
                state,
                stroke: outlined,
                evenodd,
            } => {
                if path.is_empty() {
                    return Ok(());
                }
                transform(target, state.transform);
                let brush = brush(target, state)?;
                let geometry = super::path::geometry(factory, path, *evenodd, !outlined)?;
                if *outlined {
                    target.DrawGeometry(&geometry, &brush, state.width, &stroke(factory, state)?);
                } else {
                    target.FillGeometry(&geometry, &brush, None);
                }
            }
            Draw::Text { text, point, state } => {
                if text.is_empty() {
                    return Ok(());
                }
                transform(target, state.transform);
                let layout = layout(text_engine, text, &state.font)?;
                let size = layout.size();
                let x = point[0]
                    - match state.align.as_str() {
                        "center" => size[0] / 2.,
                        "right" | "end" => size[0],
                        _ => 0.,
                    };
                let y = point[1]
                    - match state.baseline.as_str() {
                        "top" => 0.,
                        "middle" => size[1] / 2.,
                        "bottom" => size[1],
                        _ => layout.baseline(),
                    };
                layout.draw(target, &brush(target, state)?, [x, y])?;
            }
        }
        Ok(())
    }
}
pub(super) fn layout(
    engine: &crate::surface_text::Engine,
    text: &str,
    font: &str,
) -> Result<crate::surface_text::Layout> {
    let (size, family) = commands::font(font)
        .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_INVALIDARG))?;
    // Canvas 单行文本将 ASCII 空白变成普通空格，不套用 DOM 的合并空白/换行模型。
    let text: String = text
        .chars()
        .map(|ch| {
            if matches!(ch, '\n' | '\r' | '\t' | '\u{000c}') {
                ' '
            } else {
                ch
            }
        })
        .collect();
    engine.layout(
        &text,
        crate::surface_text::Style {
            family,
            size,
            weight: 400,
            whitespace: crate::surface_text::Whitespace::Pre,
            align: crate::surface_text::Align::Start,
            line_height: None,
        },
        32768.,
        32768.,
    )
}
