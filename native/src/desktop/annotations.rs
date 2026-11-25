//! 基础标注的原生呈现；坐标由视口转换，图像裁剪由调用者持有。
use super::effect_cache::Cache;
use crate::{
    editor::{Editor, Stroke, Tool},
    viewport::{Point, Viewport},
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

struct Preview<'a> {
    offset: Option<(usize, Point)>,
    draft: bool,
    replacement: Option<(usize, &'a Stroke)>,
    group: &'a [(usize, Stroke)],
    hidden: Option<usize>,
}

pub unsafe fn draw_preview(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    preview: Option<(usize, Point)>,
    replacement: Option<(usize, &Stroke)>,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    draw_preview_excluding(
        target,
        brush,
        editor,
        viewport,
        preview,
        replacement,
        None,
        cache,
    )
}

pub(super) unsafe fn draw_preview_excluding(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    preview: Option<(usize, Point)>,
    replacement: Option<(usize, &Stroke)>,
    hidden: Option<usize>,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    draw_layers(
        target,
        brush,
        editor,
        viewport,
        Preview {
            offset: preview,
            draft: true,
            replacement,
            group: &[],
            hidden,
        },
        cache,
    )
}

pub unsafe fn draw_group_preview(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    group: &[(usize, Stroke)],
    cache: &mut Cache,
) -> windows::core::Result<()> {
    draw_layers(
        target,
        brush,
        editor,
        viewport,
        Preview {
            offset: None,
            draft: true,
            replacement: None,
            group,
            hidden: None,
        },
        cache,
    )
}

pub unsafe fn draw_committed(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    draw_layers(
        target,
        brush,
        editor,
        viewport,
        Preview {
            offset: None,
            draft: false,
            replacement: None,
            group: &[],
            hidden: None,
        },
        cache,
    )
}

unsafe fn draw_layers(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    preview_state: Preview<'_>,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    let Preview {
        offset: preview,
        draft: include_draft,
        replacement,
        group,
        hidden,
    } = preview_state;
    let strokes = || {
        // All group producers build previews in sorted selection order; merge in one pass per rank.
        let mut group = group.iter().peekable();
        editor
            .elements()
            .iter()
            .enumerate()
            .filter(move |(index, _)| Some(*index) != hidden)
            .map(move |(index, stroke)| {
                let preview = group
                    .next_if(|(current, _)| *current == index)
                    .map(|(_, preview)| preview);
                (
                    index,
                    preview
                        .or_else(|| {
                            replacement
                                .filter(|(selected, _)| *selected == index)
                                .map(|(_, changed)| changed)
                        })
                        .unwrap_or(stroke),
                )
            })
            .chain(
                editor
                    .draft()
                    .filter(|_| include_draft)
                    .map(|stroke| (usize::MAX, stroke)),
            )
    };
    let (mut ranks, mut blur, mut mosaic) = (0u8, false, false);
    for (_, stroke) in strokes() {
        ranks |= 1 << stroke.tool.render_rank();
        blur |= stroke.tool == Tool::Blur;
        mosaic |= stroke.tool == Tool::Mosaic;
    }
    // 撤销/删除最后一个效果后，下一帧释放其缓存，而不是等到窗口销毁。
    cache.retain_tools(blur, mosaic);
    if blur {
        cache.retain_blurs(strokes().filter_map(|(_, stroke)| stroke.blur.as_ref()));
    }
    // 基线荧光笔统一在其他标注下方，层内涂抹不突破整体 35% 上限。
    for rank in 0..=3 {
        if ranks & (1 << rank) == 0 {
            continue;
        }
        let _wash = if rank == 0 {
            let (left, top, right, bottom) =
                super::annotation_rotation::visible_bounds(target, 0.0)?;
            let clip = crate::highlighter_bounds::Bounds {
                left,
                top,
                right,
                bottom,
            };
            let Some(bounds) =
                crate::highlighter_bounds::visible(strokes(), viewport, preview, clip)
            else {
                continue;
            };
            Some(super::annotation_opacity::push_wash(target, bounds))
        } else {
            None
        };
        for (index, stroke) in strokes() {
            if stroke.tool.render_rank() != rank {
                continue;
            }
            let mut position = viewport;
            if let Some((selected, offset)) = preview {
                if selected == index {
                    position.origin.x += offset.x * position.scale;
                    position.origin.y += offset.y * position.scale;
                }
            }
            draw_stroke(target, brush, stroke, position, cache)?;
        }
    }
    Ok(())
}

unsafe fn draw_stroke(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    if !stroke.visible_paint() || stroke.points.is_empty() {
        return Ok(());
    }
    let _rotation = super::annotation_rotation::apply(target, stroke, viewport);
    if stroke.tool == Tool::Blur {
        return super::annotation_blur::draw(target, stroke, viewport, cache);
    }
    // 遮挡工具不继承普通画笔透明度/虚线，避免共享样式意外泄露底图。
    if stroke.tool == Tool::Mosaic {
        return super::annotation_mosaic::draw(target, stroke, viewport, &mut cache.mosaic);
    }
    if stroke.tool == Tool::Paintbrush {
        return super::annotation_paintbrush::draw(target, brush, stroke, viewport);
    }
    let layer = super::annotation_opacity::push(target, stroke, viewport)?;
    let result = draw_opaque_stroke(target, brush, stroke, viewport);
    // 包括几何创建失败的路径，都必须先恢复图层栈再传播错误。
    drop(layer);
    result
}

unsafe fn draw_opaque_stroke(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) -> windows::core::Result<()> {
    let Some(first) = stroke.points.first() else {
        return Ok(());
    };
    let a = transform(*first, viewport);
    let b = transform(*stroke.points.last().unwrap(), viewport);
    let width = stroke.width * viewport.scale;
    let dash = super::annotation_dash::create(target, stroke)?;
    let crate::stroke_style::Color(red, green, blue) = stroke.color;
    brush.SetColor(&D2D1_COLOR_F {
        r: red as f32 / 255.0,
        g: green as f32 / 255.0,
        b: blue as f32 / 255.0,
        a: super::annotation_opacity::stroke_alpha(stroke),
    });
    let rect = D2D_RECT_F {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    };
    match stroke.tool {
        Tool::Mosaic | Tool::Blur => unreachable!("effects are drawn before ordinary opacity"),
        Tool::Serial => super::annotation_serial::draw(target, brush, stroke, viewport)?,
        Tool::Text => super::annotation_text::draw(target, brush, stroke, viewport)?,
        Tool::Brush | Tool::Highlighter | Tool::Polyline => {
            super::annotation_freehand::draw(target, brush, stroke, viewport, dash.as_ref())?;
        }
        Tool::Line | Tool::Arrow => {
            super::annotation_linear::draw(target, brush, stroke, viewport, dash.as_ref())?;
        }
        Tool::Bezier => {
            if stroke.arrow {
                if let Some((shaft, head)) =
                    super::annotation_bezier::arrow_paths(&target.GetFactory()?, stroke, viewport)?
                {
                    target.DrawGeometry(&shaft, brush, width, dash.as_ref());
                    target.FillGeometry(&head, brush, None);
                    return Ok(());
                }
            }
            let geometry =
                super::annotation_bezier::geometry(&target.GetFactory()?, stroke, viewport)?;
            target.DrawGeometry(&geometry, brush, width, dash.as_ref());
        }
        Tool::Rectangle | Tool::RoundedRectangle | Tool::Ellipse => super::annotation_shapes::draw(
            target,
            brush,
            stroke,
            rect,
            viewport.scale,
            dash.as_ref(),
        ),
        Tool::Pan | Tool::Select | Tool::Paintbrush | Tool::Eraser => {}
        Tool::Triangle | Tool::Polygon => {
            super::annotation_polygons::draw(target, brush, stroke, viewport, dash.as_ref())?
        }
    }
    Ok(())
}

fn transform(point: Point, viewport: Viewport) -> D2D_POINT_2F {
    D2D_POINT_2F {
        x: viewport.origin.x + point.x * viewport.scale,
        y: viewport.origin.y + point.y * viewport.scale,
    }
}
