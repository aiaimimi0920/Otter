//! 子矩形放大后再擦除，避免把细笔迹压回原图的一个像素而丢失局部覆盖。
use super::*;

pub(super) fn prepare(
    image: &Image,
    geometry: &mut Frame,
    crop: Option<Rect>,
    cancel: &AtomicBool,
) -> Result<Option<Image>> {
    if !matches!(geometry.sampling, Sampling::Region(_)) {
        return Ok(None);
    }
    let layout = Layout::for_frame(image, geometry, crop)?;
    let sampled = source_sample::readback(image, &layout, cancel)?;
    // 只在确实有笔迹时物化有预算的输出。几何交换发生在成功之后，不返回半成品。
    geometry.output = Some(layout.output);
    geometry.base = Some(D2D_RECT_F {
        left: layout.visible.x as f32,
        top: layout.visible.y as f32,
        right: (layout.visible.x + layout.visible.width) as f32,
        bottom: (layout.visible.y + layout.visible.height) as f32,
    });
    geometry.base_clip = None;
    geometry.sampling = Sampling::Full;
    Ok(Some(sampled))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_materialization_does_not_exchange_geometry_or_source_pixels() {
        let image = Image {
            width: 4,
            height: 1,
            pixels: [10, 20, 30, 255].repeat(4).into(),
        };
        let mut geometry: Frame = Size {
            width: 4.0,
            height: 1.0,
        }
        .into();
        geometry.sampling = Projection::clipped(
            Rect {
                x: 1.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            Rect {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 1.0,
            },
            (4, 1),
        )
        .unwrap();
        assert!(prepare(&image, &mut geometry, None, &AtomicBool::new(true)).is_err());
        assert!(matches!(geometry.sampling, Sampling::Region(_)));
        assert!(geometry.output.is_none() && geometry.base.is_none());
        assert!(image.pixels.chunks_exact(4).all(|p| p == [10, 20, 30, 255]));
    }
}
