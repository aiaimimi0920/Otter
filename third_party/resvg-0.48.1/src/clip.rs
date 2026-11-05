// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::render::Context;

pub fn apply(
    clip: &usvg::ClipPath,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();
    clip_pixmap.fill(tiny_skia::Color::BLACK);

    draw_children(
        clip.root(),
        ctx,
        tiny_skia::BlendMode::Clear,
        transform.pre_concat(clip.transform()),
        &mut clip_pixmap.as_mut(),
    );

    if let Some(clip) = clip.clip_path() {
        apply(clip, ctx, transform, pixmap);
    }

    let mut mask = tiny_skia::Mask::from_pixmap(clip_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    mask.invert();
    pixmap.apply_mask(&mask);
}

fn draw_children(
    parent: &usvg::Group,
    ctx: &Context,
    mode: tiny_skia::BlendMode,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for child in parent.children() {
        match child {
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }

                // Preserve the clip-local bounds while retaining the render's error owner.
                let local = Context {
                    max_bbox: tiny_skia::IntRect::from_xywh(0, 0, 1, 1).unwrap(),
                    filter_failed: ctx.filter_failed,
                    filter_work: ctx.filter_work,
                };
                crate::path::fill_path(path, mode, &local, transform, pixmap);
            }
            usvg::Node::Text(text) => {
                draw_children(text.flattened(), ctx, mode, transform, pixmap);
            }
            usvg::Node::Group(group) => {
                let transform = transform.pre_concat(group.transform());

                if let Some(clip) = group.clip_path() {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(group, clip, ctx, transform, pixmap);
                } else {
                    draw_children(group, ctx, mode, transform, pixmap);
                }
            }
            _ => {}
        }
    }
}

fn clip_group(
    children: &usvg::Group,
    clip: &usvg::ClipPath,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    draw_children(
        children,
        ctx,
        tiny_skia::BlendMode::SourceOver,
        transform,
        &mut clip_pixmap.as_mut(),
    );
    apply(clip, ctx, transform, &mut clip_pixmap);

    let mut paint = tiny_skia::PixmapPaint::default();
    paint.blend_mode = tiny_skia::BlendMode::Xor;
    pixmap.draw_pixmap(
        0,
        0,
        clip_pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );

    Some(())
}
