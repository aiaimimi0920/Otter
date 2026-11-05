//! 透明标注层的独立 WIC 合成；不画底图，不写文件，调用者负责版本守卫与事务提交。
use crate::{
    editor::Editor,
    erase_layers::Layers,
    pixels,
    viewport::{Point, Viewport},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_INVALIDARG, E_OUTOFMEMORY},
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Imaging::*,
        },
        System::Com::*,
    },
};

#[cfg(test)]
pub(super) fn render(layers: &Layers, editor: &Editor, indices: &[usize]) -> Result<Layers> {
    let snapshot = editor
        .rasterize_snapshot(indices)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid rasterization selection"))?;
    render_snapshot(layers, &snapshot)
}

/// 仅接收已筛选的无历史矢量快照；在后台线程内建立并释放全部 COM 资源。
pub(super) fn render_snapshot(layers: &Layers, snapshot: &Editor) -> Result<Layers> {
    let crop = snapshot.annotation_frame();
    let (width, height) = crop.map_or(layers.size, |rect| {
        (
            rect.width.round().max(1.0) as u32,
            rect.height.round().max(1.0) as u32,
        )
    });
    let bytes = crate::viewport::checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid rasterization dimensions"))?;
    if !layers.valid() {
        return Err(Error::new(E_INVALIDARG, "Invalid rasterization pixels"));
    }
    let _apartment = super::Apartment::new()?;
    let _backing = pixels::reserve(bytes)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Raster backing budget exceeded"))?;
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let output = wic.CreateBitmap(
            width,
            height,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapCacheOnLoad,
        )?;
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let format = D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        };
        let target = factory.CreateWicBitmapRenderTarget(
            &output,
            &D2D1_RENDER_TARGET_PROPERTIES {
                dpiX: 96.0,
                dpiY: 96.0,
                pixelFormat: format,
                ..Default::default()
            },
        )?;
        let existing = layers
            .annotations
            .as_ref()
            .map(|pixels| {
                let (aw, ah) = layers.annotation_dimensions();
                target.CreateBitmap(
                    D2D_SIZE_U {
                        width: aw,
                        height: ah,
                    },
                    Some(pixels.as_ptr().cast()),
                    aw * 4,
                    &D2D1_BITMAP_PROPERTIES {
                        pixelFormat: format,
                        dpiX: 96.0,
                        dpiY: 96.0,
                    },
                )
            })
            .transpose()?;
        let brush = target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?;
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        if let Some(bitmap) = existing {
            target.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: width as f32,
                    bottom: height as f32,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
        if let Some(rect) = crop {
            // 只对源坐标矢量应用裁剪变换；已有栅格层已经完整适配到输出框。
            let mut transform = Default::default();
            target.GetTransform(&mut transform);
            transform.M11 = width as f32 / rect.width as f32;
            transform.M22 = height as f32 / rect.height as f32;
            transform.M31 = -(rect.x as f32) * transform.M11;
            transform.M32 = -(rect.y as f32) * transform.M22;
            target.SetTransform(&transform);
        }
        let drawn = super::annotations::draw_committed(
            &target,
            &brush,
            snapshot,
            Viewport {
                origin: Point::default(),
                scale: 1.0,
            },
            &mut super::effect_cache::Cache::default(),
        );
        let finished = target.EndDraw(None, None);
        drawn?;
        finished?;
        let mut lease = pixels::reserve(bytes)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Raster output budget exceeded"))?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Raster allocation failed"))?;
        if !lease.grow_to(pixels.capacity()) {
            return Err(Error::new(E_OUTOFMEMORY, "Raster capacity budget exceeded"));
        }
        pixels.resize(bytes, 0);
        output.CopyPixels(std::ptr::null(), width * 4, &mut pixels)?;
        Ok(Layers {
            annotation_size: ((width, height) != layers.size).then_some((width, height)),
            size: layers.size,
            base: layers.base.clone(),
            annotations: Some(crate::pixels::Pixels::from_reserved(pixels, lease)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cropped_rasterization_fits_existing_texture_and_translates_selected_vector() {
        let source = Layers {
            annotation_size: Some((2, 1)),
            size: (32, 32),
            base: [30, 40, 50, 255].repeat(1024).into(),
            annotations: Some(vec![0, 255, 0, 255, 0, 0, 255, 255].into()),
        };
        let mut editor = Editor::with_layers(source.clone()).unwrap();
        editor.set_style(
            editor
                .style()
                .with_width(4.0)
                .unwrap()
                .with_color(crate::stroke_style::Color(0, 0, 255)),
        );
        for x in [20.0, 4.0] {
            editor
                .begin(crate::editor::Tool::Brush, Point { x, y: 12.0 })
                .unwrap();
            assert!(editor.commit());
        }
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 16.0,
                y: 8.0,
                width: 8.0,
                height: 8.0,
            })
            .unwrap();
        let crop = editor.crop_view();
        let point_allocation = editor.elements()[0].points.as_ptr();
        let rendered = render(&source, &editor, &[0]).unwrap();
        assert_eq!(rendered.annotation_dimensions(), (8, 8));
        assert!(rendered.base.same_storage(&source.base));
        let pixels = rendered.annotations.as_ref().unwrap();
        assert_eq!(&pixels[..4], &[0, 255, 0, 255]);
        assert_eq!(&pixels[28..32], &[0, 0, 255, 255]);
        assert_eq!(&pixels[(4 * 8 + 4) * 4..(4 * 8 + 5) * 4], &[255, 0, 0, 255]);
        // 使用正式宿主的统一刷新事务，而非只测兼容提交接口。
        let plan = editor
            .prepare_rasterize_edit(&[0])
            .unwrap()
            .unwrap()
            .with_candidate(rendered)
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.elements().len(), 1);
        assert_eq!(editor.crop_view(), crop);
        for _ in 0..3 {
            assert!(editor.undo());
            assert!(editor.layers().unwrap().same_storage(&source));
            assert_eq!(editor.layers().unwrap().annotation_dimensions(), (2, 1));
            assert_eq!(editor.elements()[0].points.as_ptr(), point_allocation);
            assert!(editor.redo());
            assert_eq!(editor.layers().unwrap().annotation_dimensions(), (8, 8));
        }
    }
    #[test]
    fn independent_texture_rasterization_without_crop_uses_full_frame() {
        let source = Layers {
            annotation_size: Some((1, 1)),
            size: (16, 8),
            base: vec![0; 512].into(),
            annotations: Some(vec![0, 128, 0, 128].into()),
        };
        let mut editor = Editor::with_layers(source.clone()).unwrap();
        editor
            .begin(crate::editor::Tool::Brush, Point { x: 100.0, y: 100.0 })
            .unwrap();
        editor.commit();
        let rendered = render(&source, &editor, &[0]).unwrap();
        assert_eq!(rendered.annotation_dimensions(), (16, 8));
        assert!(rendered
            .annotations
            .as_ref()
            .unwrap()
            .chunks_exact(4)
            .all(|p| p == [0, 128, 0, 128]));
        assert!(editor
            .commit_rasterized(editor.revision(), &source, rendered, &[0])
            .unwrap());
        assert!(editor.undo());
        assert_eq!(editor.layers().unwrap().annotation_dimensions(), (1, 1));
    }

    #[test]
    fn selected_render_preserves_existing_raster_and_excludes_base_and_other_vectors() {
        let mut existing = vec![0; 32 * 32 * 4];
        existing[..4].copy_from_slice(&[0, 255, 0, 255]);
        let source = Layers {
            annotation_size: None,
            size: (32, 32),
            base: [0, 0, 255, 255].repeat(32 * 32).into(),
            annotations: Some(existing.into()),
        };
        let mut editor = Editor::with_layers(source.clone()).unwrap();
        editor.set_style(
            editor
                .style()
                .with_width(6.0)
                .unwrap()
                .with_color(crate::stroke_style::Color(0, 0, 255)),
        );
        for x in [8.0, 24.0] {
            editor
                .begin(crate::editor::Tool::Brush, Point { x, y: 16.0 })
                .unwrap();
            assert!(editor.commit());
        }
        let rendered = render(&source, &editor, &[1]).unwrap();
        assert!(rendered.base.same_storage(&source.base));
        let pixels = rendered.annotations.as_ref().unwrap();
        assert_eq!(&pixels[..4], &[0, 255, 0, 255]);
        assert_eq!(
            &pixels[(16 * 32 + 24) * 4..(16 * 32 + 25) * 4],
            &[255, 0, 0, 255]
        );
        assert_eq!(&pixels[(16 * 32 + 8) * 4..(16 * 32 + 9) * 4], &[0; 4]);
        assert!(editor
            .commit_rasterized(editor.revision(), &source, rendered, &[1])
            .unwrap());
        assert_eq!(editor.elements().len(), 1);
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&source));
        assert_eq!(editor.elements().len(), 2);
    }
}
