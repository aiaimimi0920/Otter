//! 同一产品Tile Renderer的已提交目标读回；固定矩阵oracle，不调用WIC重画作为预期。
use super::*;
use crate::{
    desktop::{Apartment, Window},
    tile_wire::*,
};
use std::sync::Arc;
use windows::{
    core::w,
    Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};
fn window() -> Window {
    unsafe {
        let w = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Tile readback"),
                WS_POPUP,
                -30000,
                -30000,
                2,
                2,
                None,
                None,
                Some(GetModuleHandleW(None).unwrap().into()),
                None,
            )
            .unwrap(),
        );
        let _ = ShowWindow(w.0, SW_SHOWNOACTIVATE);
        w
    }
}
fn renderer(mode: usize, hwnd: HWND) -> Renderer {
    let size = D2D_SIZE_U {
        width: 2,
        height: 2,
    };
    let surface = match mode {
        0 => RenderSurface::D3d12(Box::new(
            crate::desktop::composition_d3d12::CompositionD3d12::new(hwnd, size).unwrap(),
        )),
        1 => RenderSurface::D3d11(Box::new(
            crate::desktop::composition::Composition::new(hwnd, size).unwrap(),
        )),
        _ => {
            let factory: ID2D1Factory =
                unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() };
            RenderSurface::software(&factory, size).unwrap()
        }
    };
    Renderer::from_surface(surface)
}
fn read(r: &mut Renderer, hwnd: HWND, f: Option<&Arc<Frame>>, expected: &[[u8; 4]]) {
    assert!(r.paint(f, None, hwnd, None).unwrap());
    let pixels = r.surface.readback_for_test();
    assert_eq!(pixels.size, [2, 2]);
    assert_eq!(
        pixels.bytes.chunks_exact(4).collect::<Vec<_>>(),
        expected.iter().map(|p| p.as_slice()).collect::<Vec<_>>()
    );
}
fn run(mode: usize) {
    let _com = Apartment::new().unwrap();
    let window = window();
    let mut r = renderer(mode, window.0);
    for (rotation, order) in [
        (Rotation::Zero, [1, 2, 3, 4]),
        (Rotation::Right, [2, 4, 1, 3]),
        (Rotation::Half, [4, 3, 2, 1]),
        (Rotation::Left, [3, 1, 4, 2]),
    ] {
        let f = tests::fixture(rotation);
        read(&mut r, window.0, Some(&f), &order.map(|n| [n, 0, 0, 255]));
    }
    let mut updated = Arc::try_unwrap(tests::fixture(Rotation::Zero))
        .ok()
        .unwrap();
    let image = updated.images.values_mut().next().unwrap();
    image.pixels = [20, 30, 40, 255].repeat(4).into();
    let updated = Arc::new(updated);
    read(&mut r, window.0, Some(&updated), &[[20, 30, 40, 255]; 4]);
    read(&mut r, window.0, None, &[[0, 0, 0, 255]; 4]);
    read(&mut r, window.0, Some(&updated), &[[20, 30, 40, 255]; 4]);
    for (rotation, order) in [
        (Rotation::Zero, [1, 2, 3, 4]),
        (Rotation::Right, [2, 4, 1, 3]),
        (Rotation::Half, [4, 3, 2, 1]),
        (Rotation::Left, [3, 1, 4, 2]),
    ] {
        let f = art_tests::frame(rotation);
        read(&mut r, window.0, Some(&f), &order.map(|n| [0, 0, n, 255]));
    }
    let mut f = Arc::try_unwrap(art_tests::frame(Rotation::Zero))
        .ok()
        .unwrap();
    f.layout.placements[0].source_crop = Rect {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    };
    f.layout.placements[0].rect.width = 1.;
    read(
        &mut r,
        window.0,
        Some(&Arc::new(f)),
        &[
            [0, 0, 2, 255],
            [0, 0, 0, 255],
            [0, 0, 4, 255],
            [0, 0, 0, 255],
        ],
    );
    let mut f = Arc::try_unwrap(art_tests::frame(Rotation::Zero))
        .ok()
        .unwrap();
    f.layout.placements[0].source_crop = Rect {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    };
    f.layout.placements[0].rect.width = 1.;
    f.images.insert(
        "upper".into(),
        crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels: vec![0, 128, 0, 128].into(),
        },
    );
    let mut upper = f.layout.placements[0].clone();
    upper.placement_id = "upper".into();
    upper.source = Source {
        kind: "image".into(),
        id: "upper".into(),
    };
    upper.z_index = 1;
    upper.rect.width = 2.;
    upper.source_crop = Rect {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
    };
    f.layout.placements.push(upper);
    read(
        &mut r,
        window.0,
        Some(&Arc::new(f)),
        &[
            [0, 128, 1, 255],
            [0, 128, 0, 255],
            [0, 128, 2, 255],
            [0, 128, 0, 255],
        ],
    );
}
#[test]
fn d3d12_presented_tile_backbuffers_hold_latest_image_art_crop_alpha_and_black_resume() {
    run(0)
}
#[test]
fn d3d11_presented_tile_backbuffer_holds_latest_image_art_crop_alpha_and_black_resume() {
    run(1)
}
#[test]
fn cpu_submitted_tile_dib_holds_latest_image_art_crop_alpha_and_black_resume() {
    run(2)
}
