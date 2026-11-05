//! Chromium Canvas Display-P3 的相同编码字节，验证真实 child 的 sRGB 与 alpha。
use super::*;
use base64::Engine;
#[path = "surface_webp_lut_tests.rs"]
mod lut_tests;
impl Image {
    pub(crate) fn webp_p3_for_test() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(RGB)
            .unwrap()
    }
}
const RGB: &str = "UklGRlICAABXRUJQVlA4WAoAAAAgAAAAAwAAAAAASUNDUAgCAAAAAAIIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAAGRyWFlaAAABVAAAABRnWFlaAAABaAAAABRiWFlaAAABfAAAABR3dHB0AAABkAAAABRyVFJDAAABpAAAAChnVFJDAAABpAAAAChiVFJDAAABpAAAAChjcHJ0AAABzAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAEYAAAAcAEQAaQBzAHAAbABhAHkAIABQADMAIABHAGEAbQB1AHQAIAB3AGkAdABoACAAcwBSAEcAQgAgAFQAcgBhAG4AcwBmAGUAcgAAWFlaIAAAAAAAAIPeAAA9vv///7tYWVogAAAAAAAASr4AALE2AAAKuVhZWiAAAAAAAAAoOwAAEQwAAMjNWFlaIAAAAAAAAPbWAAEAAAAA0y1wYXJhAAAAAAAEAAAAAmZmAADypwAADVkAABPQAAAKWwAAAAAAAAAAbWx1YwAAAAAAAAABAAAADGVuVVMAAAAgAAAAHABHAG8AbwBnAGwAZQAgAEkAbgBjAC4AIAAyADAAMQA2VlA4TCQAAAAvAwAAAB8gECAEivJkViCQYo45WiCQ2Cb4gPMfM1h46GRE/wM=";
const RGBA: &str = "UklGRlQCAABXRUJQVlA4WAoAAAAwAAAAAwAAAAAASUNDUAgCAAAAAAIIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAAGRyWFlaAAABVAAAABRnWFlaAAABaAAAABRiWFlaAAABfAAAABR3dHB0AAABkAAAABRyVFJDAAABpAAAAChnVFJDAAABpAAAAChiVFJDAAABpAAAAChjcHJ0AAABzAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAEYAAAAcAEQAaQBzAHAAbABhAHkAIABQADMAIABHAGEAbQB1AHQAIAB3AGkAdABoACAAcwBSAEcAQgAgAFQAcgBhAG4AcwBmAGUAcgAAWFlaIAAAAAAAAIPeAAA9vv///7tYWVogAAAAAAAASr4AALE2AAAKuVhZWiAAAAAAAAAoOwAAEQwAAMjNWFlaIAAAAAAAAPbWAAEAAAAA0y1wYXJhAAAAAAAEAAAAAmZmAADypwAADVkAABPQAAAKWwAAAAAAAAAAbWx1YwAAAAAAAAABAAAADGVuVVMAAAAgAAAAHABHAG8AbwBnAGwAZQAgAEkAbgBjAC4AIAAyADAAMQA2VlA4TCYAAAAvAwAAEB8gECAEulLkliDbNuUxi+0XCBC1FPI3gPFdkGHQyYj+Bw==";

#[test]
fn surface_webp_icc_p3_matches_chromium_srgb_and_alpha() {
    for (encoded, expected) in [
        (
            RGB,
            [
                188, 124, 27, 255, 0, 182, 93, 255, 110, 47, 212, 255, 127, 127, 127, 255,
            ],
        ),
        (
            RGBA,
            [
                183, 126, 28, 128, 0, 179, 94, 128, 110, 48, 209, 128, 128, 128, 128, 128,
            ],
        ),
    ] {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let image = Image::from_surface(&bytes, "image/webp", None, || false).unwrap();
        assert_eq!((image.width, image.height), (4, 1));
        for (actual, expected) in image.pixels.chunks_exact(4).zip(expected.chunks_exact(4)) {
            assert_eq!(actual[3], expected[3]);
            for (channel, source) in [(0, 2), (1, 1), (2, 0)] {
                let target =
                    ((u16::from(expected[source]) * u16::from(expected[3]) + 127) / 255) as u8;
                assert!(
                    actual[channel].abs_diff(target) <= 2,
                    "{actual:?} vs {expected:?}"
                );
            }
        }
    }
}

#[test]
fn surface_webp_icc_missing_invalid_and_non_rgb_match_browser_identity() {
    let original = base64::engine::general_purpose::STANDARD
        .decode(RGB)
        .unwrap();
    let at = 30;
    assert_eq!(&original[at..at + 4], b"ICCP");
    let len = u32::from_le_bytes(original[at + 4..at + 8].try_into().unwrap()) as usize;
    for mode in 0..4 {
        let mut metadata = original[at + 8..at + 8 + len].to_vec();
        match mode {
            1 => metadata[36] = b'X',
            2 => metadata = vec![1],
            3 => metadata[16..20].copy_from_slice(b"CMYK"),
            _ => {}
        }
        let mut bytes = original[..at].to_vec();
        if mode == 0 {
            bytes[20] &= !32;
        } else {
            bytes.extend_from_slice(b"ICCP");
            bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&metadata);
            if !metadata.len().is_multiple_of(2) {
                bytes.push(0);
            }
        }
        bytes.extend_from_slice(&original[at + 8 + len..]);
        let size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        let image = Image::from_surface(&bytes, "image/webp", None, || false).unwrap();
        assert_eq!(
            &*image.pixels,
            &[51, 127, 179, 255, 102, 179, 51, 255, 204, 51, 102, 255, 127, 127, 127, 255],
            "mode={mode}"
        );
    }
}
