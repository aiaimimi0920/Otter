//! WIC/D2D 实际栅格像素；预期是手工标号矩阵，不调用新几何算法生成 oracle。
use super::*;
use crate::tile_wire::*;
use windows::Win32::{Graphics::Imaging::*, System::Com::*};
pub(super) fn fixture(rotation: Rotation) -> std::sync::Arc<Frame> {
    let (_, mut frame) = crate::tile_tests::shared_frame();
    frame.layout.tiles[0].rotation = rotation;
    let id = format!("sha256:{}", "a".repeat(64));
    frame.images.insert(
        id.clone(),
        crate::image_decode::Image {
            width: 2,
            height: 2,
            pixels: vec![1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255].into(),
        },
    );
    frame.layout.placements.push(Placement {
        placement_id: "p".into(),
        source: Source {
            kind: "image".into(),
            id,
        },
        rect: Rect {
            x: 0.,
            y: 0.,
            width: 2.,
            height: 2.,
        },
        source_crop: Rect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
        },
        z_index: 0,
        interactive: false,
    });
    std::sync::Arc::new(frame)
}
pub(super) unsafe fn render(frame: &std::sync::Arc<Frame>) -> Vec<u8> {
    render_local(frame, &[])
}
pub(super) unsafe fn render_local(
    frame: &std::sync::Arc<Frame>,
    rows: &[crate::tile_surface_local::Entry],
) -> Vec<u8> {
    let wic: IWICImagingFactory = windows::Win32::System::Com::CoCreateInstance(
        &CLSID_WICImagingFactory,
        None,
        CLSCTX_INPROC_SERVER,
    )
    .unwrap();
    let bitmap = wic
        .CreateBitmap(
            frame.size.width,
            frame.size.height,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapCacheOnLoad,
        )
        .unwrap();
    let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
    let target = factory
        .CreateWicBitmapRenderTarget(
            &bitmap,
            &D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.,
                dpiY: 96.,
                ..Default::default()
            },
        )
        .unwrap();
    let mut images = BTreeMap::new();
    for (id, image) in &frame.images {
        images.insert(
            id.clone(),
            target
                .CreateBitmap(
                    D2D_SIZE_U {
                        width: image.width,
                        height: image.height,
                    },
                    Some(image.pixels.as_ptr().cast()),
                    image.width * 4,
                    &D2D1_BITMAP_PROPERTIES {
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        dpiX: 96.,
                        dpiY: 96.,
                    },
                )
                .unwrap(),
        );
    }
    target.BeginDraw();
    let mut arts = BTreeMap::new();
    art::prepare(&mut arts, Some(frame)).unwrap();
    art::local::apply(&mut arts, Some(frame), rows).unwrap();
    paint(&target, Some(frame), &images, &arts, None).unwrap();
    target.EndDraw(None, None).unwrap();
    let mut result = vec![0; (frame.size.width * frame.size.height * 4) as usize];
    bitmap
        .CopyPixels(std::ptr::null(), frame.size.width * 4, &mut result)
        .unwrap();
    result
}
#[test]
fn four_rotations_match_hand_numbered_source_pixels() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    for (rotation, expected) in [
        (Rotation::Zero, [1, 2, 3, 4]),
        (Rotation::Right, [2, 4, 1, 3]),
        (Rotation::Half, [4, 3, 2, 1]),
        (Rotation::Left, [3, 1, 4, 2]),
    ] {
        let actual = unsafe { render(&fixture(rotation)) };
        assert_eq!(
            actual.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>(),
            expected
        );
    }
    unsafe {
        CoUninitialize();
    }
}
#[test]
fn cropped_nonuniform_placement_and_alpha_z_order_are_applied_once() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let mut f = std::sync::Arc::try_unwrap(fixture(Rotation::Zero))
        .ok()
        .unwrap();
    f.layout.placements[0].source_crop = Rect {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    };
    f.layout.placements[0].rect.width = 1.;
    let actual = unsafe { render(&std::sync::Arc::new(f)) };
    assert_eq!(
        actual.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>(),
        [2, 0, 4, 0]
    );
    let mut f = std::sync::Arc::try_unwrap(fixture(Rotation::Zero))
        .ok()
        .unwrap();
    let id = format!("sha256:{}", "b".repeat(64));
    f.images.insert(
        id.clone(),
        crate::image_decode::Image {
            width: 2,
            height: 2,
            pixels: [0, 0, 128, 128].repeat(4).into(),
        },
    );
    let mut upper = f.layout.placements[0].clone();
    upper.placement_id = "upper".into();
    upper.source.id = id;
    upper.z_index = 1;
    f.layout.placements.push(upper);
    let actual = unsafe { render(&std::sync::Arc::new(f)) };
    assert_eq!(&actual[..4], &[0, 0, 128, 255]);
    assert_eq!(actual[6], 128);
    assert_eq!(actual[7], 255);
    unsafe {
        CoUninitialize();
    }
}
