//! 独立矩形覆盖oracle：320x240逻辑Art框中contain正方形，再仅用原WIC转换取整。
//! 不调用生产来源、布局、D2D或投送合成器；正式PNG自身不应带逻辑留白。
use windows::Win32::{Graphics::Imaging::*, System::Com::*};

pub(crate) const LOGICAL: (u32, u32) = (320, 240);

pub(super) fn transmitted(color: [u8; 4]) -> Vec<u8> {
    assert_eq!(color[3], 255);
    let (width, height) = (f64::from(LOGICAL.0), f64::from(LOGICAL.1));
    let side = width.min(height);
    let (left, top) = ((width - side) / 2.0, (height - side) / 2.0);
    let (pixel_width, pixel_height) = (width / 2.0, height / 2.0);
    let mut bgra = Vec::with_capacity(16);
    for y in 0..2 {
        for x in 0..2 {
            let overlap = |start: f64, length: f64, edge: f64| {
                ((start + length).min(edge + side) - start.max(edge)).max(0.0)
            };
            let area = overlap(f64::from(x) * pixel_width, pixel_width, left)
                * overlap(f64::from(y) * pixel_height, pixel_height, top)
                / (pixel_width * pixel_height);
            bgra.extend(
                [color[2], color[1], color[0], color[3]]
                    .map(|c| (f64::from(c) * area).round() as u8),
            );
        }
    }
    straight(&bgra)
}

fn straight(bgra: &[u8]) -> Vec<u8> {
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        let _apartment = Apartment;
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmapFromMemory(2, 2, &GUID_WICPixelFormat32bppPBGRA, 8, bgra)
            .unwrap();
        let converter = wic.CreateFormatConverter().unwrap();
        converter
            .Initialize(
                &bitmap,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .unwrap();
        let mut output = vec![0; 16];
        converter
            .CopyPixels(std::ptr::null(), 8, &mut output)
            .unwrap();
        output
    }
}

#[test]
fn independent_contain_coverage_uses_declared_logical_frame_and_confirmed_wic_rounding() {
    // 面积为3/4，预乘通道/alpha独立算为191；WIC旧合同的straight结果为254/191。
    assert_eq!(transmitted([255, 0, 0, 255]), [254, 0, 0, 191].repeat(4));
    assert_eq!(transmitted([0, 255, 0, 255]), [0, 254, 0, 191].repeat(4));
}
