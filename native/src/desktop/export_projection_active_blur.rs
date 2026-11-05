//! 活动效果按自己的当前采样框重建；不借 canonical 的旧标注框，也不递归采样矢量。
use super::*;

pub(in crate::desktop) fn prepare_active(
    image: &Image,
    editor: &Editor,
    geometry: &Frame,
    cancel: &AtomicBool,
) -> Result<Editor> {
    check(cancel)?;
    if !editor.elements().iter().any(|stroke| stroke.blur.is_some()) {
        return Ok(editor.export_snapshot());
    }
    let parent = geometry.layout(image, editor)?;
    let mut cache = HashMap::<[u64; 12], BlurImage>::new();
    editor.export_snapshot_with_blur_strokes(|stroke| {
        check(cancel)?;
        let original = stroke.blur.as_ref().unwrap();
        let frame = original.frame();
        let clip = original.effect_clip();
        let mask = original.effect_mask_scale();
        let c = clip.unwrap_or(frame);
        let key = [
            frame.x.to_bits(),
            frame.y.to_bits(),
            frame.width.to_bits(),
            frame.height.to_bits(),
            original.strength().to_bits() as u64,
            clip.is_some() as u64,
            c.x.to_bits(),
            c.y.to_bits(),
            c.width.to_bits(),
            c.height.to_bits(),
            mask.0.to_bits() as u64,
            mask.1.to_bits() as u64,
        ];
        if let Some(prepared) = cache.get(&key) {
            return Ok(prepared.clone());
        }
        let prepared = prepare(image, editor, geometry, &parent, original, cancel)?;
        cache.try_reserve(1).map_err(|_| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Cannot cache active blur texture",
            )
        })?;
        cache.insert(key, prepared.clone());
        Ok(prepared)
    })
}

fn prepare(
    image: &Image,
    editor: &Editor,
    geometry: &Frame,
    parent: &Layout,
    original: &BlurImage,
    cancel: &AtomicBool,
) -> Result<BlurImage> {
    let frame = original.frame();
    let output = (
        (frame.width * parent.output.0 as f64 / parent.visible.width).ceil() as u32,
        (frame.height * parent.output.1 as f64 / parent.visible.height).ceil() as u32,
    );
    checked_image_bytes(output.0, output.1)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Active blur exceeds pixel limits"))?;
    // 复用最终底图的传播/legacy Region/Empty布局，而非重新contain整张自然图。
    // visible换成效果完整采样框，保留crop外邻域；仅底图自己的内容clip仍需保留。
    let mut layout = Layout {
        output,
        visible: frame,
        base: parent.base,
        // 与当前viewport完全相同的clip只负责最终crop，不能截断卷积邻域。
        // 真正较小/偏移的传播内容clip仍属于底图，不能覆盖到独立栅格标注。
        base_clip: parent.base_clip.filter(|clip| {
            editor.crop_view().is_none()
                || clip.left != parent.visible.x as f32
                || clip.top != parent.visible.y as f32
                || clip.right != (parent.visible.x + parent.visible.width) as f32
                || clip.bottom != (parent.visible.y + parent.visible.height) as f32
        }),
        sampling: parent.sampling,
        interpolation: D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        border: None,
        radius: 0.0,
    };
    let clip = original.effect_clip().map(|clip| D2D_RECT_F {
        left: clip.x as f32,
        top: clip.y as f32,
        right: (clip.x + clip.width) as f32,
        bottom: (clip.y + clip.height) as f32,
    });
    let has_overlay = editor
        .layers()
        .is_some_and(|layers| layers.annotations.is_some());
    if !has_overlay {
        layout.base_clip = intersection(layout.base_clip, clip);
    }
    let sampled = source_sample::readback(image, &layout, cancel)?;
    let sx = output.0 as f64 / frame.width;
    let sy = output.1 as f64 / frame.height;
    let layers = editor.layers();
    let overlay = layers.and_then(|layers| layers.annotations.clone());
    let annotation_size = overlay
        .as_ref()
        .and_then(|_| layers.map(|layers| layers.annotation_dimensions()));
    let visible = editor.annotation_frame().unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: geometry.logical.width,
        height: geometry.logical.height,
    });
    let composed = crate::raster_source::compose_in_frame(
        &crate::erase_layers::Layers {
            size: output,
            base: sampled.pixels,
            annotations: overlay,
            annotation_size,
        },
        Some(Rect {
            x: (visible.x - frame.x) * sx,
            y: (visible.y - frame.y) * sy,
            width: visible.width * sx,
            height: visible.height * sy,
        }),
        || cancel.load(Ordering::Acquire),
    )
    .map_err(failure)?;
    // clip限制所有取样颜色而非只有底图；否则clip外的栅格标注会卷积渗入clip内。
    let composed = if has_overlay && clip.is_some() {
        let layout = Layout {
            output,
            visible: frame,
            base: D2D_RECT_F {
                left: frame.x as f32,
                top: frame.y as f32,
                right: (frame.x + frame.width) as f32,
                bottom: (frame.y + frame.height) as f32,
            },
            base_clip: clip,
            sampling: Sampling::Full,
            interpolation: D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            border: None,
            radius: 0.0,
        };
        source_sample::readback(
            &Image {
                width: output.0,
                height: output.1,
                pixels: composed,
            },
            &layout,
            cancel,
        )?
        .pixels
    } else {
        composed
    };
    let pixels = crate::blur::region_scaled(
        &composed,
        output,
        crate::blur::Region {
            x: 0,
            y: 0,
            width: output.0,
            height: output.1,
        },
        original.strength(),
        (sx as f32, sy as f32),
        || cancel.load(Ordering::Acquire),
    )
    .map_err(failure)?;
    let mut prepared = BlurImage::with_strength(output.0, output.1, pixels, original.strength())
        .and_then(|image| image.with_frame(frame))
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid active blur texture"))?;
    if let Some(clip) = original.effect_clip() {
        prepared = prepared
            .with_effect_clip(clip)
            .and_then(|image| image.with_effect_mask_scale(original.effect_mask_scale()))
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid active blur clip"))?;
    }
    check(cancel)?;
    Ok(prepared)
}

fn intersection(a: Option<D2D_RECT_F>, b: Option<D2D_RECT_F>) -> Option<D2D_RECT_F> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let left = a.left.max(b.left);
            let top = a.top.max(b.top);
            Some(D2D_RECT_F {
                left,
                top,
                right: a.right.min(b.right).max(left),
                bottom: a.bottom.min(b.bottom).max(top),
            })
        }
        (a, b) => a.or(b),
    }
}
