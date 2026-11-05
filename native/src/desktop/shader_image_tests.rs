//! 完整浏览器 alpha 矩阵、真实 WIC PNG 与失败原子性；不访问用户桌面或文件。
use super::*;
use crate::byte_budget::Budget;
use sha2::{Digest, Sha256};
use std::cell::Cell;

pub(crate) const STRAIGHT_SHA256: &str =
    "7a6d7767fc6e3bc96bdd47ea10c4340915e3a7af30c48b9f4e5ed8a20bfc5ef4";

pub(crate) fn matrix() -> Frame {
    let mut bytes = Vec::with_capacity(256 * 256 * 4);
    for alpha in 0..=255u8 {
        for red in 0..=255u8 {
            bytes.extend_from_slice(&[
                red,
                255 - red,
                red.wrapping_mul(73).wrapping_add(17),
                alpha,
            ]);
        }
    }
    Frame {
        width: 256,
        height: 256,
        rgba: bytes.into(),
    }
}
fn tiny() -> Frame {
    Frame {
        width: 1,
        height: 1,
        rgba: vec![64, 32, 16, 128].into(),
    }
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn decode_rgba(png: &[u8], width: u32, height: u32) -> Vec<u8> {
    use windows::Win32::{Graphics::Imaging::*, System::Com::*};
    let _apartment = super::super::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = wic.CreateStream().unwrap();
        stream.InitializeFromMemory(png).unwrap();
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
            .unwrap();
        let frame = decoder.GetFrame(0).unwrap();
        let (mut actual_width, mut actual_height) = (0, 0);
        frame
            .GetSize(&mut actual_width, &mut actual_height)
            .unwrap();
        assert_eq!((actual_width, actual_height), (width, height));
        let converter = wic.CreateFormatConverter().unwrap();
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .unwrap();
        let mut bytes = vec![0; width as usize * height as usize * 4];
        converter
            .CopyPixels(std::ptr::null(), width * 4, &mut bytes)
            .unwrap();
        bytes
    }
}

#[test]
fn full_alpha_matrix_matches_actual_browser_png_not_integer_unpremultiplication() {
    let source = matrix();
    assert_eq!(
        hash(&source.rgba),
        "6a26422f3d2bbc33e7ebd143a32f008fafd2c397601c397f1f75bc950e2f56bb"
    );
    let original = source.rgba.clone();
    let (display, straight) = convert(source, &|| false, crate::pixels::reserve).unwrap();
    let mut rgba = straight.to_vec();
    for ((raw, straight), display) in original
        .chunks_exact(4)
        .zip(rgba.chunks_exact_mut(4))
        .zip(display.pixels.chunks_exact(4))
    {
        straight.swap(0, 2);
        assert_eq!(
            display,
            &[
                raw[2].min(raw[3]),
                raw[1].min(raw[3]),
                raw[0].min(raw[3]),
                raw[3]
            ]
        );
        for channel in 0..3 {
            assert_eq!(
                (u16::from(straight[channel]) * u16::from(raw[3]) + 127) / 255,
                u16::from(raw[channel].min(raw[3]))
            );
        }
    }
    // 摘要来自当前旧 ShaderRenderer → canvas.toBlob PNG，由独立 Pillow 解码，不是自算 oracle。
    assert_eq!(hash(&rgba), STRAIGHT_SHA256);
    assert_eq!(&rgba[(6 * 256 + 1) * 4..][..1], &[42]);
    assert_eq!(&rgba[(10 * 256 + 3) * 4..][..1], &[77]);
}

#[test]
fn wic_png_preserves_browser_straight_bytes_and_native_display_for_all_alpha_pairs() {
    let output = prepare(matrix(), &AtomicBool::new(false)).unwrap();
    let rgba = decode_rgba(&output.png, 256, 256);
    assert_eq!(hash(&rgba), STRAIGHT_SHA256);
    let _apartment = super::super::Apartment::new().unwrap();
    let restored = super::super::export::decode_png(output.png.to_vec());
    assert_eq!(restored.pixels, output.image.pixels);
}

#[test]
fn invalid_geometry_and_early_cancellation_do_not_reserve_or_encode() {
    for (width, height, bytes) in [
        (0, 1, 4),
        (8193, 1, 4),
        (8192, 8192, 4),
        (2, 1, 4),
        (1, 1, 3),
    ] {
        let frame = Frame {
            width,
            height,
            rgba: vec![0; bytes].into(),
        };
        let result = prepare_with(
            frame,
            &AtomicBool::new(false),
            |_| panic!("reserved invalid input"),
            |_, _, _, _| panic!("encoded invalid input"),
        );
        assert_eq!(result.err().unwrap().code(), E_INVALIDARG);
    }
    let result = prepare_with(
        tiny(),
        &AtomicBool::new(true),
        |_| panic!("reserved cancelled input"),
        |_, _, _, _| panic!("encoded cancelled input"),
    );
    assert_eq!(result.err().unwrap().code(), E_ABORT);
}

#[test]
fn either_allocation_failure_and_mid_conversion_cancellation_release_both_leases() {
    for limit in [0, 4, 7] {
        let budget = Budget::new(limit);
        let source = tiny();
        let weak = source.rgba.downgrade();
        let result = prepare_with(
            source,
            &AtomicBool::new(false),
            |n| budget.reserve(n),
            |_, _, _, _| panic!("encoded incomplete pixels"),
        );
        assert_eq!(result.err().unwrap().code(), E_OUTOFMEMORY);
        assert_eq!(budget.used(), 0);
        assert!(weak.upgrade().is_none());
    }
    let source = matrix();
    let budget = Budget::new(source.rgba.len() * 2);
    let checks = Cell::new(0);
    let result = convert(
        source,
        &|| {
            checks.set(checks.get() + 1);
            checks.get() == 3
        },
        |n| budget.reserve(n),
    );
    assert_eq!(result.err().unwrap().code(), E_ABORT);
    assert_eq!(checks.get(), 3);
    assert_eq!(budget.used(), 0);
}

#[test]
fn encoding_failure_or_late_cancellation_cannot_return_partial_result() {
    let budget = Budget::new(8);
    let cancel = AtomicBool::new(false);
    let result = prepare_with(
        tiny(),
        &cancel,
        |n| budget.reserve(n),
        |width, height, bytes, _| {
            assert_eq!(
                (width, height, bytes.as_ref()),
                (1, 1, &[32, 64, 128, 128][..])
            );
            Err(Error::new(E_OUTOFMEMORY, "injected encoding failure"))
        },
    );
    assert_eq!(result.err().unwrap().code(), E_OUTOFMEMORY);
    assert_eq!(budget.used(), 0);
    let encoded: Pixels = vec![1].into();
    let weak = encoded.downgrade();
    let result = prepare_with(
        tiny(),
        &cancel,
        |n| budget.reserve(n),
        |_, _, _, _| {
            cancel.store(true, Ordering::Release);
            Ok(encoded)
        },
    );
    assert_eq!(result.err().unwrap().code(), E_ABORT);
    assert!(weak.upgrade().is_none());
    assert_eq!(budget.used(), 0);
}
