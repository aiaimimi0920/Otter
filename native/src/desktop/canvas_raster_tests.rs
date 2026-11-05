//! GPU 层身份、原子上传和宿主撤销后的栅格视图同步。
use super::*;
use windows::{
    core::Interface,
    Win32::{Graphics::Imaging::*, System::Com::*},
};

#[test]
fn legacy_base_mapping_renders_source_region_through_real_d2d_target() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(8, 8, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let pixels: Pixels = (0..64)
            .flat_map(|i| {
                let v = if (1..=4).contains(&(i % 8)) { 64 } else { 192 };
                [v, v, v, 255]
            })
            .collect::<Vec<_>>()
            .into();
        let image = Image {
            width: 8,
            height: 8,
            pixels: pixels.clone(),
        };
        let size = crate::crop_geometry::Size {
            width: 8.0,
            height: 8.0,
        };
        let editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (8, 8),
            base: pixels,
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_size(size)
        .unwrap()
        .with_restored_legacy_crop(
            crate::crop_geometry::Rect {
                x: 2.0,
                y: 0.0,
                width: 2.0,
                height: 8.0,
            },
            size,
        )
        .unwrap();
        let mapping = editor.raster_base_projection().unwrap();
        let r = mapping.target;
        let rect = D2D_RECT_F {
            left: r.x as f32,
            top: r.y as f32,
            right: (r.x + r.width) as f32,
            bottom: (r.y + r.height) as f32,
        };
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 8.0,
            bottom: 8.0,
        };
        let mut resources = Resources::default();
        resources
            .sync_sized(&target, Some(&image), None, (0, 0))
            .unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        resources
            .draw_erasing_projected(
                &target,
                &rect,
                &clip,
                None,
                Viewport {
                    origin: Point::default(),
                    scale: 1.0,
                },
                Some(mapping),
            )
            .unwrap();
        target.EndDraw(None, None).unwrap();
        let mut bytes = vec![0; 8 * 8 * 4];
        output
            .CopyPixels(std::ptr::null(), 8 * 4, &mut bytes)
            .unwrap();
        assert_eq!(bytes, [64, 64, 64, 255].repeat(64));
    }
}

#[test]
fn annotation_destination_does_not_change_base_destination_or_reupload() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(8, 8, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let image = Image {
            width: 8,
            height: 8,
            pixels: [100, 0, 0, 255].repeat(64).into(),
        };
        let overlay: Pixels = vec![0, 0, 200, 255, 0, 200, 0, 255].into();
        let mut resources = Resources::default();
        resources
            .sync_sized(&target, Some(&image), Some(&overlay), (2, 1))
            .unwrap();
        let texture = resources.annotations.as_ref().unwrap().bitmap.clone();
        let rect = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 8.0,
            bottom: 8.0,
        };
        let cropped = D2D_RECT_F {
            left: 4.0,
            top: 2.0,
            right: 8.0,
            bottom: 6.0,
        };
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        resources
            .draw_erasing(&target, &rect, &cropped, None, Viewport::default())
            .unwrap();
        target.EndDraw(None, None).unwrap();
        let mut bytes = vec![0; 8 * 8 * 4];
        output
            .CopyPixels(std::ptr::null(), 8 * 4, &mut bytes)
            .unwrap();
        assert_eq!(&bytes[0..4], &[100, 0, 0, 255]);
        assert_eq!(&bytes[(3 * 8 + 4) * 4..(3 * 8 + 5) * 4], &[0, 0, 200, 255]);
        assert_eq!(&bytes[(3 * 8 + 7) * 4..(3 * 8 + 8) * 4], &[0, 200, 0, 255]);
        resources
            .sync_sized(&target, Some(&image), Some(&overlay), (2, 1))
            .unwrap();
        assert_eq!(
            texture.as_raw(),
            resources.annotations.as_ref().unwrap().bitmap.as_raw()
        );
        let base = resources.base.as_ref().unwrap().bitmap.clone();
        resources
            .sync_sized(&target, Some(&image), Some(&overlay), (1, 2))
            .unwrap();
        assert_ne!(
            texture.as_raw(),
            resources.annotations.as_ref().unwrap().bitmap.as_raw()
        );
        assert_eq!(
            base.as_raw(),
            resources.base.as_ref().unwrap().bitmap.as_raw()
        );
        assert_eq!(resources.annotations.as_ref().unwrap().size, (1, 2));
    }
}

#[test]
fn erase_preview_reveals_base_or_transparency_without_reuploading_layers() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let image = Image {
            width: 32,
            height: 32,
            pixels: [0, 0, 255, 255].repeat(32 * 32).into(),
        };
        let overlay: Pixels = [0, 255, 0, 255].repeat(32 * 32).into();
        let mut resources = Resources::default();
        resources
            .sync(&target, Some(&image), Some(&overlay))
            .unwrap();
        let base = resources.base.as_ref().unwrap().bitmap.clone();
        let annotations = resources.annotations.as_ref().unwrap().bitmap.clone();
        let rect = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 32.0,
            bottom: 32.0,
        };
        let viewport = Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        for mode in [
            crate::erase_layers::Mode::Content,
            crate::erase_layers::Mode::RasterAnnotations,
        ] {
            let request = crate::erase_job::Request {
                preview_identity: std::sync::Arc::new(()),
                annotation_frame: None,
                base_projection: None,
                source: Layers {
                    annotation_size: None,
                    size: (32, 32),
                    base: image.pixels.clone(),
                    annotations: Some(overlay.clone()),
                },
                revision: 0,
                mode,
                points: vec![Point { x: 12.0, y: 16.0 }, Point { x: 20.0, y: 16.0 }],
                width: 12.0,
            };
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            resources
                .draw_erasing(&target, &rect, &rect, Some(&request), viewport)
                .unwrap();
            target.EndDraw(None, None).unwrap();
            let mut pixels = vec![0; 32 * 32 * 4];
            output
                .CopyPixels(std::ptr::null(), 32 * 4, &mut pixels)
                .unwrap();
            assert_eq!(&pixels[..4], &[0, 255, 0, 255]);
            let center = (16 * 32 + 16) * 4;
            let expected = if mode == crate::erase_layers::Mode::Content {
                [0; 4]
            } else {
                [0, 0, 255, 255]
            };
            assert_eq!(&pixels[center..center + 4], &expected);
            assert_eq!(
                base.as_raw(),
                resources.base.as_ref().unwrap().bitmap.as_raw()
            );
            assert_eq!(
                annotations.as_raw(),
                resources.annotations.as_ref().unwrap().bitmap.as_raw()
            );
        }
        // 取消只移除预览遮罩，原有纹理不需要重新上传。
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        resources
            .draw_erasing(&target, &rect, &rect, None, viewport)
            .unwrap();
        target.EndDraw(None, None).unwrap();
        let mut pixels = vec![0; 32 * 32 * 4];
        output
            .CopyPixels(std::ptr::null(), 32 * 4, &mut pixels)
            .unwrap();
        assert_eq!(
            &pixels[(16 * 32 + 16) * 4..(16 * 32 + 17) * 4],
            &[0, 255, 0, 255]
        );
    }
}

#[test]
fn uploads_only_changed_layer_and_draws_raster_above_base() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(16, 16, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let image = Image {
            width: 16,
            height: 16,
            pixels: [0, 0, 255, 255].repeat(16 * 16).into(),
        };
        let overlay: Pixels = [0, 255, 0, 255].repeat(16 * 16).into();
        let mut resources = Resources::default();
        resources
            .sync(&target, Some(&image), Some(&overlay))
            .unwrap();
        let base = resources.base.as_ref().unwrap().bitmap.clone();
        let annotation = resources.annotations.as_ref().unwrap().bitmap.clone();
        resources
            .sync(&target, Some(&image), Some(&overlay.clone()))
            .unwrap();
        assert_eq!(
            base.as_raw(),
            resources.base.as_ref().unwrap().bitmap.as_raw()
        );
        assert_eq!(
            annotation.as_raw(),
            resources.annotations.as_ref().unwrap().bitmap.as_raw()
        );
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        resources.draw(
            &target,
            &D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 16.0,
            },
            &D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 16.0,
            },
        );
        target.EndDraw(None, None).unwrap();
        let mut result = vec![0; 16 * 16 * 4];
        output
            .CopyPixels(std::ptr::null(), 16 * 4, &mut result)
            .unwrap();
        assert_eq!(&result[..4], &[0, 255, 0, 255]);
        let new_image = Image {
            pixels: vec![0; 16 * 16 * 4].into(),
            ..image.clone()
        };
        let invalid: Pixels = vec![0; 4].into();
        assert!(resources
            .sync(&target, Some(&new_image), Some(&invalid))
            .is_err());
        assert_eq!(
            base.as_raw(),
            resources.base.as_ref().unwrap().bitmap.as_raw()
        );
        assert_eq!(
            annotation.as_raw(),
            resources.annotations.as_ref().unwrap().bitmap.as_raw()
        );
        resources
            .sync(&target, Some(&new_image), Some(&overlay))
            .unwrap();
        assert_ne!(
            base.as_raw(),
            resources.base.as_ref().unwrap().bitmap.as_raw()
        );
        assert_eq!(
            annotation.as_raw(),
            resources.annotations.as_ref().unwrap().bitmap.as_raw()
        );
        resources.sync(&target, Some(&new_image), None).unwrap();
        assert!(resources.annotations.is_none());
    }
}

#[test]
fn canvas_syncs_erase_undo_redo_and_invalid_replacement_keeps_document() {
    let image = Image {
        width: 16,
        height: 16,
        pixels: vec![255; 16 * 16 * 4].into(),
    };
    let mut canvas = Canvas::new(Some(image.clone())).unwrap();
    let original = canvas.editor.layers().unwrap().clone();
    let erased = original
        .erase(
            crate::erase_layers::Mode::Content,
            &[Point { x: 8.0, y: 8.0 }],
            8.0,
            || false,
        )
        .unwrap();
    canvas.blur_ready = Some((
        original.clone(),
        crate::blur_image::BlurImage::new(16, 16, image.pixels.clone(), 8).unwrap(),
        None,
        None,
    ));
    assert!(canvas
        .editor
        .replace_layers(0, &original, erased.clone())
        .unwrap());
    canvas.sync_raster_source();
    assert!(canvas
        .image
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&erased.base));
    assert!(canvas.blur_ready.is_none());
    assert!(canvas.editor.undo());
    canvas.sync_raster_source();
    assert!(canvas
        .image
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&original.base));
    assert!(canvas.editor.redo());
    canvas.sync_raster_source();
    assert!(canvas
        .image
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&erased.base));
    let revision = canvas.editor.revision();
    assert!(canvas
        .replace_image(Image {
            width: 16,
            height: 16,
            pixels: vec![0; 4].into()
        })
        .is_err());
    assert_eq!(canvas.editor.revision(), revision);
    assert!(canvas.editor.layers().unwrap().same_storage(&erased));
}
