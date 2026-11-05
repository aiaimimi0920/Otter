//! 局部大纹理通过完整 blur 绘制入口，与相同几何/裁切的整图路径对照。
use super::*;
use crate::{blur_image::BlurImage, crop_geometry::Rect, editor::Editor, viewport::Point};
use windows::Win32::{
    Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
    System::Com::*,
};

unsafe fn target() -> (IWICBitmap, ID2D1RenderTarget) {
    let wic: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
    let output = wic
        .CreateBitmap(
            800,
            220,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapCacheOnLoad,
        )
        .unwrap();
    let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
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
    (output, target)
}
fn stroke() -> Stroke {
    let size = (2052, 96);
    let pixels = (0..size.0 * size.1)
        .flat_map(|i| {
            let x = i % size.0;
            [(x / 16 % 128) as u8, (i / size.0) as u8, 80, 200]
        })
        .collect::<Vec<_>>();
    let pixels = crate::blur::region(
        &pixels.into(),
        size,
        crate::blur::Region {
            x: 0,
            y: 0,
            width: size.0,
            height: size.1,
        },
        4,
        || false,
    )
    .unwrap();
    let frame = Rect {
        x: 12.5,
        y: -3.25,
        width: 512.25,
        height: 47.25,
    };
    let image = BlurImage::new(size.0, size.1, pixels, 4)
        .unwrap()
        .with_frame(frame)
        .unwrap()
        .with_effect_clip(Rect {
            x: 100.0,
            y: 0.0,
            width: 300.0,
            height: 36.0,
        })
        .unwrap()
        .with_effect_mask_scale(((512.25 / 513.0) as f32, (47.25 / 48.0) as f32))
        .unwrap();
    let mut editor = Editor::default();
    editor.begin_blur(Point { x: 30.0, y: 5.0 }, image).unwrap();
    editor.extend(Point { x: 220.0, y: 30.0 }).unwrap();
    editor.extend(Point { x: 500.0, y: 3.0 }).unwrap();
    assert!(editor.commit());
    let mut stroke = editor.elements()[0].clone();
    stroke.width = 24.0;
    stroke
}
unsafe fn render(
    output: &IWICBitmap,
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
    cache: &mut super::super::effect_cache::Cache,
    mode: Option<bool>,
) -> Vec<u8> {
    target.BeginDraw();
    target.Clear(Some(&D2D1_COLOR_F::default()));
    {
        let _rotation = super::super::annotation_rotation::apply(target, stroke, viewport);
        draw_impl(target, stroke, viewport, cache, mode).unwrap();
    }
    target.EndDraw(None, None).unwrap();
    let mut pixels = vec![0; 800 * 220 * 4];
    output
        .CopyPixels(std::ptr::null(), 800 * 4, &mut pixels)
        .unwrap();
    pixels
}

#[test]
fn local_blur_tiles_match_full_drawing_with_fractional_masks_and_rotated_source_clips() {
    let _com = super::super::Apartment::new().unwrap();
    unsafe {
        let (output, target) = target();
        let mut stroke = stroke();
        let mut cache = super::super::effect_cache::Cache::default();
        for (scale, angle) in [(1.0, 0.0), (1.25, 17.0), (0.75, -23.0)] {
            stroke.rotation = angle;
            let viewport = Viewport {
                origin: Point { x: 10.25, y: 65.5 },
                scale,
            };
            let mut reference = super::super::effect_cache::Cache::default();
            let expected = render(
                &output,
                &target,
                &stroke,
                viewport,
                &mut reference,
                Some(false),
            );
            assert!(expected.chunks_exact(4).any(|p| p[3] > 40));
            for pass in 0..2 {
                let uploads = cache.blur_uploads;
                let actual = render(&output, &target, &stroke, viewport, &mut cache, None);
                if pass == 1 {
                    assert_eq!(cache.blur_uploads, uploads);
                }
                for (index, (a, b)) in actual.iter().zip(&expected).enumerate() {
                    assert!(
                        a.abs_diff(*b) <= 1,
                        "scale={scale} angle={angle} byte={index} tiled={a} full={b}"
                    );
                }
            }
        }
        assert!(
            cache.blur_uploads <= 5,
            "local draw uploaded more than the fixed source grid"
        );
    }
}
