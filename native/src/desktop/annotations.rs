//! Native rendering for basic annotations; the viewport transforms coordinates and the caller owns image clipping.
use super::effect_cache::Cache;
use crate::{
    editor::{Editor, Stroke, Tool},
    viewport::{Point, Viewport},
};
use std::collections::HashSet;
use windows::Win32::Graphics::Direct2D::{Common::*, *};
#[path = "annotation_magnifier.rs"]
pub(super) mod magnifier;
#[path = "annotation_spotlight.rs"]
pub(super) mod spotlight;
use magnifier::Pass;

#[derive(Clone, Copy, Default)]
pub(super) struct Preview<'a> {
    pub offset: Option<(usize, Point)>,
    pub draft: bool,
    pub replacement: Option<(usize, &'a Stroke)>,
    pub group: &'a [(usize, Stroke)],
    pub hidden: Option<usize>,
}

#[cfg(test)]
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

#[cfg(test)]
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
        Pass::Plain,
    )
}

#[cfg(test)]
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
        Pass::Plain,
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
        Pass::Plain,
    )
}

pub(super) unsafe fn draw_layers(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    editor: &Editor,
    viewport: Viewport,
    preview_state: Preview<'_>,
    cache: &mut Cache,
    pass: Pass<'_>,
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
                let replacement = preview.or_else(|| {
                    replacement
                        .filter(|(selected, _)| *selected == index)
                        .map(|(_, changed)| changed)
                });
                (index, replacement.unwrap_or(stroke), replacement.is_none())
            })
            .chain(
                editor
                    .draft()
                    .filter(|_| include_draft)
                    .map(|stroke| (usize::MAX, stroke, false)),
            )
    };
    let (mut ranks, mut blur, mut mosaic) = (0u8, false, false);
    let mut active_ids = HashSet::new();
    for (_, stroke, _) in strokes() {
        ranks |= 1 << stroke.tool.render_rank();
        blur |= stroke.tool == Tool::Blur;
        mosaic |= stroke.tool == Tool::Mosaic;
    }
    // Previews retain official IDs but must not replace committed geometry caches; committed entries also survive hiding/transforms.
    active_ids.extend(editor.elements().iter().map(Stroke::annotation_id));
    // After undoing/deleting the last effect, release its cache on the next frame rather than waiting for window destruction.
    cache.retain_tools(blur, mosaic);
    if blur {
        cache.retain_blurs(strokes().filter_map(|(_, stroke, _)| stroke.blur.as_ref()));
    }
    cache.annotations.retain_active(&active_ids);
    // Baseline highlighters all sit below other annotations; overpainting within the layer never exceeds the overall 35% cap.
    for rank in 0..=5 {
        if pass.lower() && rank > 2 {
            break;
        }
        if ranks & (1 << rank) == 0 {
            continue;
        }
        if rank == 3 {
            spotlight::draw(
                target,
                brush,
                editor.spotlight_percent(),
                strokes()
                    .filter(|(_, s, _)| s.tool.spotlight())
                    .map(|(index, stroke, _)| {
                        let mut vp = viewport;
                        if let Some((_, offset)) = preview.filter(|(i, _)| *i == index) {
                            vp.origin.x += offset.x * vp.scale;
                            vp.origin.y += offset.y * vp.scale;
                        }
                        (stroke, vp)
                    }),
            )?;
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
            let Some(bounds) = crate::highlighter_bounds::visible(
                strokes().map(|(index, stroke, _)| (index, stroke)),
                viewport,
                preview,
                clip,
            ) else {
                continue;
            };
            Some(super::annotation_opacity::push_wash(target, bounds))
        } else {
            None
        };
        for (index, stroke, cacheable) in strokes() {
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
            if rank == 4 {
                magnifier::draw(target, pass, stroke, position)?;
            } else {
                draw_stroke(target, brush, stroke, position, cacheable, cache)?;
            }
        }
    }
    Ok(())
}

unsafe fn draw_stroke(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
    cacheable: bool,
    cache: &mut Cache,
) -> windows::core::Result<()> {
    if !stroke.visible_paint() || stroke.points.is_empty() {
        return Ok(());
    }
    let _rotation = super::annotation_rotation::apply(target, stroke, viewport);
    if stroke.tool == Tool::Blur {
        return super::annotation_blur::draw(target, stroke, viewport, cache);
    }
    // Obscuring tools do not inherit ordinary brush opacity/dashes, preventing shared styles from accidentally exposing the base image.
    if stroke.tool == Tool::Mosaic {
        return super::annotation_mosaic::draw(target, stroke, viewport, &mut cache.mosaic);
    }
    if stroke.tool == Tool::Paintbrush {
        return super::annotation_paintbrush::draw_cached(
            target,
            brush,
            stroke,
            viewport,
            cacheable,
            &mut cache.annotations,
        );
    }
    let layer = super::annotation_opacity::push(target, stroke, viewport)?;
    let result = draw_opaque_stroke(
        target,
        brush,
        stroke,
        viewport,
        cacheable,
        &mut cache.annotations,
    );
    // Restore the layer stack before propagating errors, including geometry-creation failures.
    drop(layer);
    result
}

unsafe fn draw_opaque_stroke(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
    cacheable: bool,
    cache: &mut super::effect_cache::annotation_cache::Cache,
) -> windows::core::Result<()> {
    let Some(first) = stroke.points.first() else {
        return Ok(());
    };
    let a = transform(*first, viewport);
    let b = transform(*stroke.points.last().unwrap(), viewport);
    let width = stroke.width * viewport.scale;
    let dash = super::annotation_dash::cached(target, stroke, cache)?;
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
        Tool::SpotRect | Tool::SpotRounded | Tool::SpotEllipse | Tool::Magnifier => {
            unreachable!("document effects use staged composition")
        }
        Tool::Mosaic | Tool::Blur => unreachable!("effects are drawn before ordinary opacity"),
        Tool::Serial => super::annotation_serial::draw(target, brush, stroke, viewport)?,
        Tool::Text => {
            super::annotation_text::draw_cached(target, brush, stroke, viewport, cacheable, cache)?
        }
        Tool::Brush | Tool::Highlighter | Tool::Polyline => {
            super::annotation_freehand::draw_cached(
                target,
                brush,
                stroke,
                viewport,
                dash.as_ref(),
                cacheable,
                cache,
            )?;
        }
        Tool::Line | Tool::Arrow => {
            super::annotation_linear::draw(target, brush, stroke, viewport, dash.as_ref())?;
        }
        Tool::Bezier => {
            super::annotation_bezier::draw_cached(
                target,
                brush,
                stroke,
                viewport,
                width,
                dash.as_ref(),
                cacheable,
                cache,
            )?;
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
        Tool::Triangle | Tool::Polygon => super::annotation_polygons::draw_cached(
            target,
            brush,
            stroke,
            viewport,
            dash.as_ref(),
            cacheable,
            cache,
        )?,
    }
    Ok(())
}

fn transform(point: Point, viewport: Viewport) -> D2D_POINT_2F {
    D2D_POINT_2F {
        x: viewport.origin.x + point.x * viewport.scale,
        y: viewport.origin.y + point.y * viewport.scale,
    }
}
