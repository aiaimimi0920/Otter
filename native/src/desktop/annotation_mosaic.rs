//! 马赛克路径遮罩与可见区域分块；不读取底图，不按整条路径跨度分配纹理。
use crate::{
    editor::Stroke,
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
    cache: &mut super::mosaic_cache::Cache,
) -> Result<()> {
    let Some(first) = stroke.points.first() else {
        return Ok(());
    };
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    for p in &stroke.points {
        left = left.min(p.x);
        top = top.min(p.y);
        right = right.max(p.x);
        bottom = bottom.max(p.y);
    }
    left = (left - stroke.width / 2.0).floor();
    top = (top - stroke.width / 2.0).floor();
    right = (right + stroke.width / 2.0).ceil();
    bottom = (bottom + stroke.width / 2.0).ceil();
    let visible = super::annotation_rotation::visible_bounds(target, 1.0)?;
    left = left.max(((visible.0 - viewport.origin.x) / viewport.scale).floor());
    top = top.max(((visible.1 - viewport.origin.y) / viewport.scale).floor());
    right = right.min(((visible.2 - viewport.origin.x) / viewport.scale).ceil());
    bottom = bottom.min(((visible.3 - viewport.origin.y) / viewport.scale).ceil());
    if right <= left || bottom <= top {
        return Ok(());
    }
    let convert = |x, y| D2D_POINT_2F {
        x: viewport.origin.x + x * viewport.scale,
        y: viewport.origin.y + y * viewport.scale,
    };
    let a = convert(left, top);
    let b = convert(right, bottom);
    let mask = mask(target, stroke, viewport)?;
    let _layer = super::annotation_opacity::push_mask(
        target,
        D2D_RECT_F {
            left: a.x,
            top: a.y,
            right: b.x,
            bottom: b.y,
        },
        mask.clone(),
    );
    let factory = target.GetFactory()?;
    // CPU 暂存每次不超过 256×256；路径遮罩只建立一次。
    // 目标私有缓存保留最多 8 块；D2D/驱动仍可能持有提交前的内部资源。
    for y in (top as i32..bottom as i32).step_by(256) {
        for x in (left as i32..right as i32).step_by(256) {
            let width = (right as i32 - x).min(256) as u32;
            let height = (bottom as i32 - y).min(256) as u32;
            let a = convert(x as f32, y as f32);
            let b = convert(x as f32 + width as f32, y as f32 + height as f32);
            // 保守扩出 1 DIP，覆盖几何扁平化误差与抗锯齿边缘；只跳过确定不相交的块。
            let bounds = factory.CreateRectangleGeometry(&D2D_RECT_F {
                left: a.x - 1.0,
                top: a.y - 1.0,
                right: b.x + 1.0,
                bottom: b.y + 1.0,
            })?;
            if mask.CompareWithGeometry(&bounds, None, 0.25)? == D2D1_GEOMETRY_RELATION_DISJOINT {
                continue;
            }
            let bitmap = cache.bitmap(target, x, y, width, height, stroke.mosaic_cell)?;
            target.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: a.x,
                    top: a.y,
                    right: b.x,
                    bottom: b.y,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
                None,
            );
        }
    }
    Ok(())
}

pub(super) unsafe fn mask(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<ID2D1Geometry> {
    mask_points(target, &stroke.points, stroke.width, viewport)
}

pub(super) unsafe fn mask_points(
    target: &ID2D1RenderTarget,
    points: &[Point],
    width: f32,
    viewport: Viewport,
) -> Result<ID2D1Geometry> {
    use windows::core::Interface;
    let factory = target.GetFactory()?;
    let convert = |p: Point| D2D_POINT_2F {
        x: viewport.origin.x + p.x * viewport.scale,
        y: viewport.origin.y + p.y * viewport.scale,
    };
    if points.len() == 1 {
        return factory
            .CreateEllipseGeometry(&D2D1_ELLIPSE {
                point: convert(points[0]),
                radiusX: width * viewport.scale / 2.0,
                radiusY: width * viewport.scale / 2.0,
            })?
            .cast();
    }
    let path = factory.CreatePathGeometry()?;
    let sink = path.Open()?;
    sink.BeginFigure(convert(points[0]), D2D1_FIGURE_BEGIN_HOLLOW);
    for p in &points[1..] {
        sink.AddLine(convert(*p));
    }
    sink.EndFigure(D2D1_FIGURE_END_OPEN);
    sink.Close()?;
    let style = factory.CreateStrokeStyle(
        &D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            ..Default::default()
        },
        None,
    )?;
    let widened = factory.CreatePathGeometry()?;
    let sink = widened.Open()?;
    path.Widen(width * viewport.scale, &style, None, 0.25, &sink)?;
    sink.Close()?;
    widened.cast()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Editor, Tool};
    use windows::Win32::{
        Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
        System::Com::*,
    };
    #[test]
    fn sparse_path_skips_empty_tiles_and_repaint_reuses_uploads_without_missing_ink() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let output = wic
                .CreateBitmap(
                    700,
                    700,
                    &GUID_WICPixelFormat32bppPBGRA,
                    WICBitmapCacheOnLoad,
                )
                .unwrap();
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let target = factory
                .CreateWicBitmapRenderTarget(
                    &output,
                    &D2D1_RENDER_TARGET_PROPERTIES {
                        dpiX: 96.0,
                        dpiY: 96.0,
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
            let mut editor = Editor::default();
            editor.set_style(editor.style().with_width(8.0).unwrap());
            editor
                .begin(Tool::Mosaic, Point { x: 10.0, y: 10.0 })
                .unwrap();
            editor.extend(Point { x: 690.0, y: 690.0 }).unwrap();
            editor.commit();
            let viewport = Viewport {
                origin: Point::default(),
                scale: 1.0,
            };
            let mut cache = super::super::mosaic_cache::Cache::default();
            for pass in 0..2 {
                let previous_uploads = cache.uploads;
                target.BeginDraw();
                target.Clear(Some(&D2D1_COLOR_F::default()));
                draw(&target, &editor.elements()[0], viewport, &mut cache).unwrap();
                target.EndDraw(None, None).unwrap();
                assert!(
                    cache.uploads >= 3 && cache.uploads < 9,
                    "uploads: {}",
                    cache.uploads
                );
                if pass == 1 {
                    assert_eq!(cache.uploads, previous_uploads);
                }
            }
            let mut pixels = vec![0; 700 * 700 * 4];
            output
                .CopyPixels(std::ptr::null(), 700 * 4, &mut pixels)
                .unwrap();
            for coordinate in 10..690 {
                assert_eq!(
                    pixels[(coordinate * 700 + coordinate) * 4 + 3],
                    255,
                    "diagonal {coordinate}"
                );
            }
            assert_eq!(pixels[(20 * 700 + 650) * 4 + 3], 0);
        }
    }
}
