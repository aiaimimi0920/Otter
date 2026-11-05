//! 实际 WIC 目标对照整图绘制，覆盖块接缝、分数缩放和有界可见上传。
use super::*;
use crate::viewport::Point;
use windows::{
    core::Interface,
    Win32::{
        Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
        System::Com::*,
    },
};

#[test]
fn tiled_blur_matches_full_bitmap_across_seams_and_reuses_visible_tiles() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(
                800,
                180,
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
        let pixels = (0..2050 * 64)
            .flat_map(|i| {
                let x = i % 2050;
                [
                    (x % 256) as u8,
                    if x % 2 == 0 { 255 } else { 0 },
                    (i / 2050 * 3) as u8,
                    255,
                ]
            })
            .collect::<Vec<_>>();
        let image = BlurImage::new(2050, 64, pixels.into(), 8).unwrap();
        let blurred = BlurImage::new(
            2050,
            64,
            crate::blur::region(
                image.pixels(),
                (2050, 64),
                crate::blur::Region {
                    x: 0,
                    y: 0,
                    width: 2050,
                    height: 64,
                },
                8,
                || false,
            )
            .unwrap(),
            8,
        )
        .unwrap();
        let bounds = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 800.0,
            bottom: 180.0,
        };
        let mask: ID2D1Geometry = factory
            .CreateRectangleGeometry(&D2D_RECT_F {
                left: -4096.0,
                top: -4096.0,
                right: 4096.0,
                bottom: 4096.0,
            })
            .unwrap()
            .cast()
            .unwrap();
        let mut cache = Cache::default();
        for (scale, x, angle, mapped) in [
            (1.0, -450.0, 0.0f32, false),
            (1.25, -550.25, 0.0, false),
            (0.375, -4.125, 0.0, false),
            (1.0, -450.0, 17.0, false),
            (0.75, -20.0, 0.0, true),
        ] {
            // 非旋转使用高频对抗纹理；旋转对照真实高斯输出，包含透明边界。
            // D2D 对旋转高频四边形的子像素采样随四边形分割有差异，不宣称任意原图逐字节等价。
            let image = if angle == 0.0 { &image } else { &blurred };
            let mapped_image = mapped.then(|| {
                image
                    .clone()
                    .with_frame(crate::crop_geometry::Rect {
                        x: 12.5,
                        y: -3.25,
                        width: 1025.0,
                        height: 96.0,
                    })
                    .unwrap()
            });
            let image = mapped_image.as_ref().unwrap_or(image);
            let frame = image.frame();
            let (sin, cos) = angle.to_radians().sin_cos();
            let mut matrix = Default::default();
            target.GetTransform(&mut matrix);
            matrix.M11 = cos;
            matrix.M12 = sin;
            matrix.M21 = -sin;
            matrix.M22 = cos;
            matrix.M31 = 400.0 - 400.0 * cos + 90.0 * sin;
            matrix.M32 = 90.0 - 400.0 * sin - 90.0 * cos;
            target.SetTransform(&matrix);
            let visible = crate::rotation::bounds(
                (bounds.left, bounds.top, bounds.right, bounds.bottom),
                Point { x: 400.0, y: 90.0 },
                -angle,
            );
            let visible = D2D_RECT_F {
                left: visible.0,
                top: visible.1,
                right: visible.2,
                bottom: visible.3,
            };
            let viewport = Viewport {
                origin: Point { x, y: 3.25 },
                scale,
            };
            let origin_x = x + frame.x as f32 * scale;
            let origin_y = viewport.origin.y + frame.y as f32 * scale;
            let scale_x = frame.width as f32 / image.width() as f32 * scale;
            let scale_y = frame.height as f32 / image.height() as f32 * scale;
            let mut reference = Cache::default();
            let bitmap = reference.blur(&target, image).unwrap();
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            target.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: origin_x,
                    top: origin_y,
                    right: origin_x + frame.width as f32 * scale,
                    bottom: origin_y + frame.height as f32 * scale,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            target.EndDraw(None, None).unwrap();
            let mut expected = vec![0; 800 * 180 * 4];
            output
                .CopyPixels(std::ptr::null(), 800 * 4, &mut expected)
                .unwrap();
            for pass in 0..2 {
                let previous = cache.blur_uploads;
                target.BeginDraw();
                target.Clear(Some(&D2D1_COLOR_F::default()));
                draw(&target, image, viewport, visible, &mask, &mut cache).unwrap();
                target.EndDraw(None, None).unwrap();
                if pass == 1 {
                    assert_eq!(cache.blur_uploads, previous);
                }
                let mut actual = vec![0; expected.len()];
                output
                    .CopyPixels(std::ptr::null(), 800 * 4, &mut actual)
                    .unwrap();
                for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
                    let px = (i / 4 % 800) as f32 + 0.5 - 400.0;
                    let py = (i / 4 / 800) as f32 + 0.5 - 90.0;
                    let source_x = (px * cos + py * sin + 400.0 - origin_x) / scale_x;
                    let source_y = (-px * sin + py * cos + 90.0 - origin_y) / scale_y;
                    let outer_edge = source_x.abs().min((source_x - image.width() as f32).abs())
                        < 1.0 / scale
                        || source_y.abs().min((source_y - image.height() as f32).abs())
                            < 1.0 / scale;
                    // 旋转后的外轮廓由不同四边形光栅化，允许一个屏幕像素带内的覆盖率量化差；
                    // 内部块接缝和其他所有像素仍必须满足单量化级误差。
                    let tolerance = if angle != 0.0 && outer_edge { 16 } else { 1 };
                    assert!(
                        a.abs_diff(*b) <= tolerance,
                        "scale={scale} byte={i} tiled={a} full={b}"
                    );
                }
            }
        }
        // 固定网格在不同足迹桶中分别复用；缩小时增加邻域，不能复用过窄的上传区。
        assert_eq!(cache.blur_uploads, 11);
        let previous = cache.blur_uploads;
        for rect in [
            [0, 0, 0, 1],
            [2050, 0, 1, 1],
            [0, 64, 1, 1],
            [u32::MAX, 0, 2, 1],
        ] {
            assert!(cache.blur_region(&target, &image, rect).is_err());
        }
        assert_eq!(cache.blur_uploads, previous);
    }
}
