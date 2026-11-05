//! 实际 WIC/D2D 像素验证两层独立取景，而非仅断言布局元数据。
use super::*;
use crate::erase_layers::Layers;
use windows::Win32::{
    Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
    System::Com::*,
};

#[test]
fn independent_views_draw_pixels_and_keep_document_unchanged() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(4, 4, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let brush = target
            .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
            .unwrap();
        let image = Image {
            width: 8,
            height: 4,
            pixels: [100, 0, 0, 255].repeat(32).into(),
        };
        let mut overlay = vec![0; 8 * 4 * 4];
        for y in 0..4 {
            for x in 6..8 {
                overlay[(y * 8 + x) * 4..(y * 8 + x + 1) * 4].copy_from_slice(&[0, 0, 200, 255]);
            }
        }
        let editor = Editor::default()
            .with_restored_layers(Layers {
                size: (8, 4),
                base: image.pixels.clone(),
                annotations: Some(overlay.into()),
                annotation_size: None,
            })
            .unwrap();
        let stamp = editor.document_stamp();
        let views = Views {
            image: View {
                size: Size {
                    width: 8.0,
                    height: 4.0,
                },
                offset: (0.0, 0.0),
            },
            annotations: View {
                size: Size {
                    width: 8.0,
                    height: 4.0,
                },
                offset: (4.0, 0.0),
            },
        };
        let presentation = Presentation::restored(
            Some(image),
            views,
            Size {
                width: 4.0,
                height: 4.0,
            },
            &editor,
        )
        .unwrap();
        let mut resources = raster::Resources::default();
        let mut effects = Default::default();
        for dpi in [96, 144, 192] {
            presentation
                .draw(
                    &target,
                    &mut resources,
                    &brush,
                    &mut effects,
                    &editor,
                    (
                        D2D_SIZE_U {
                            width: 4,
                            height: 4,
                        },
                        dpi,
                    ),
                )
                .unwrap();
            let mut pixels = vec![0; 64];
            output
                .CopyPixels(std::ptr::null(), 16, &mut pixels)
                .unwrap();
            assert_eq!(&pixels[16..20], &[100, 0, 0, 255]);
            assert_eq!(&pixels[28..32], &[0, 0, 200, 255]);
            let mut transform = Default::default();
            target.GetTransform(&mut transform);
            assert_eq!(transform.M11, 1.0);
            assert_eq!(transform.M31, 0.0);
        }
        assert!(stamp.matches(&editor));
        let external_editor = Editor::default();
        let external_image = Image {
            width: 8,
            height: 4,
            pixels: [0, 180, 0, 255].repeat(32).into(),
        };
        let external = presentation
            .for_display(&external_image, &external_editor)
            .unwrap();
        assert!(external
            .image()
            .unwrap()
            .pixels
            .same_storage(&external_image.pixels));
        external
            .draw(
                &target,
                &mut resources,
                &brush,
                &mut effects,
                &external_editor,
                (
                    D2D_SIZE_U {
                        width: 4,
                        height: 4,
                    },
                    144,
                ),
            )
            .unwrap();
        let mut external_pixels = vec![0; 64];
        output
            .CopyPixels(std::ptr::null(), 16, &mut external_pixels)
            .unwrap();
        assert!(external_pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 180, 0, 255]));
        assert!(stamp.matches(&editor));
        let transparent = Image {
            width: 8,
            height: 4,
            pixels: vec![0; 128].into(),
        };
        let mut scaled_editor = Editor::with_layers(Layers {
            size: (8, 4),
            base: transparent.pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap();
        let red = crate::stroke_style::Color(255, 0, 0);
        scaled_editor.set_style(
            crate::stroke_style::StrokeStyle::new(1.0, red)
                .unwrap()
                .with_fill(Some(red)),
        );
        scaled_editor
            .begin(Tool::Rectangle, Point { x: 2.0, y: 0.5 })
            .unwrap();
        scaled_editor.extend(Point { x: 3.5, y: 2.5 }).unwrap();
        scaled_editor.commit();
        let view = View {
            size: Size {
                width: 16.0,
                height: 4.0,
            },
            offset: (4.0, 0.0),
        };
        let scaled = Presentation::restored(
            Some(transparent),
            Views {
                image: view,
                annotations: view,
            },
            Size {
                width: 4.0,
                height: 4.0,
            },
            &scaled_editor,
        )
        .unwrap();
        scaled
            .draw(
                &target,
                &mut resources,
                &brush,
                &mut effects,
                &scaled_editor,
                (
                    D2D_SIZE_U {
                        width: 4,
                        height: 4,
                    },
                    144,
                ),
            )
            .unwrap();
        let mut pixels = vec![0; 64];
        output
            .CopyPixels(std::ptr::null(), 16, &mut pixels)
            .unwrap();
        assert_eq!(&pixels[20..24], &[0, 0, 255, 255]);
        assert_eq!(&pixels[60..64], &[0; 4]);
        assert!(presentation
            .draw(
                &target,
                &mut resources,
                &brush,
                &mut effects,
                &Editor::default(),
                (
                    D2D_SIZE_U {
                        width: 4,
                        height: 4
                    },
                    96
                )
            )
            .is_err());
    }
}
