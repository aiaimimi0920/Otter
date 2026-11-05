//! 仅内存 DIB 夹具；覆盖 padding、行方向、掩码、截断和透明度，不读剪贴板。
use super::*;

fn compressed(bits: u16, width: i32, height: i32, stream: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for index in 0..16 {
        body.extend_from_slice(&[index, 0, 0, 0]);
    }
    body.extend_from_slice(stream);
    let mut input = dib(
        40,
        width,
        height,
        bits,
        if bits == 8 { 1 } else { 2 },
        &body,
    );
    set(&mut input, 32, 16);
    input
}

#[test]
fn rle_encoded_absolute_padding_delta_and_row_order() {
    for (bits, stream) in [
        (8, vec![3, 2, 0, 0, 0, 3, 1, 2, 3, 0, 0, 1]),
        (4, vec![3, 0x22, 0, 0, 0, 3, 0x12, 0x30, 0, 1]),
    ] {
        let image = decode(&compressed(bits, 3, 2, &stream)).unwrap();
        let values: Vec<_> = image
            .pixels
            .chunks_exact(4)
            .map(|p| {
                assert_eq!(p[3], 255);
                p[0]
            })
            .collect();
        assert_eq!(values, [1, 2, 3, 2, 2, 2]);
    }
    let image = decode(&compressed(4, 5, 1, &[5, 0x12, 0, 1])).unwrap();
    assert_eq!(
        image
            .pixels
            .chunks_exact(4)
            .map(|p| p[0])
            .collect::<Vec<_>>(),
        [1, 2, 1, 2, 1]
    );
    let image = decode(&compressed(8, 3, 2, &[0, 2, 1, 1, 1, 3, 0, 1])).unwrap();
    assert_eq!(
        image
            .pixels
            .chunks_exact(4)
            .map(|p| p[0])
            .collect::<Vec<_>>(),
        [0, 3, 0, 0, 0, 0]
    );
}

#[test]
fn rle_cancellation_during_commands_returns_no_partial_document() {
    let input = compressed(8, 3, 1, &[1, 1, 1, 2, 1, 3, 0, 1]);
    let checks = std::cell::Cell::new(0);
    let result = decode_with_cancel(&input, || {
        checks.set(checks.get() + 1);
        checks.get() == 4
    });
    assert_eq!(result.err().unwrap().code(), E_ABORT);
    assert_eq!(checks.get(), 4);
}

#[test]
fn rle_rejects_overrun_truncation_bad_index_and_top_down() {
    for bits in [4, 8] {
        for stream in [
            vec![],
            vec![1],
            vec![0, 2, 1],
            vec![4, 1, 0, 1],
            vec![0, 2, 4, 0, 0, 1],
            vec![0, 2, 0, 2, 0, 1],
            vec![0, 3, 1],
            vec![0, 0, 0, 0, 0, 0, 0, 1],
        ] {
            assert!(
                decode(&compressed(bits, 3, 2, &stream)).is_err(),
                "bits={bits} stream={stream:?}"
            );
        }
        assert!(decode(&compressed(bits, 3, -2, &[0, 1])).is_err());
    }
    assert!(decode(&compressed(8, 3, 2, &[1, 16, 0, 1])).is_err());
    // RLE4 五个绝对像素需要三个数据字节和一个填充字节。
    assert!(decode(&compressed(4, 5, 1, &[0, 5, 0x12, 0x34, 0x50])).is_err());
    let image = decode(&compressed(4, 5, 1, &[0, 5, 0x12, 0x34, 0x50, 0, 0, 1])).unwrap();
    assert_eq!(
        image
            .pixels
            .chunks_exact(4)
            .map(|p| p[0])
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
}

#[test]
fn indexed_pixels_use_high_bits_first_ignore_padding_and_reserved_alpha() {
    for (bits, packed) in [
        (1, vec![0x60, 99, 99, 99]),
        (4, vec![0x01, 0x10, 99, 99]),
        (8, vec![0, 1, 1, 99]),
    ] {
        let mut body = vec![10, 20, 30, 0, 40, 50, 60, 128];
        body.extend_from_slice(&packed);
        let mut input = dib(40, 3, -1, bits, 0, &body);
        set(&mut input, 32, 2);
        assert_eq!(
            &decode(&input).unwrap().pixels[..],
            &[10, 20, 30, 255, 40, 50, 60, 255, 40, 50, 60, 255]
        );
        for size in 40..input.len() {
            assert!(decode(&input[..size]).is_err());
        }
    }
}

#[test]
fn indexed_default_palette_and_bottom_up_rows_are_supported() {
    let input = dib(
        40,
        1,
        2,
        1,
        0,
        &[1, 2, 3, 0, 4, 5, 6, 0, 0x80, 0, 0, 0, 0, 0, 0, 0],
    );
    assert_eq!(
        &decode(&input).unwrap().pixels[..],
        &[1, 2, 3, 255, 4, 5, 6, 255]
    );
    for bits in [4, 8] {
        let colors = 1usize << bits;
        let mut body = vec![0; colors * 4 + 4];
        body[(colors - 1) * 4..colors * 4].copy_from_slice(&[7, 8, 9, 0]);
        body[colors * 4] = 255;
        let input = dib(124, 1, -1, bits, 0, &body);
        assert_eq!(&decode(&input).unwrap().pixels[..], &[7, 8, 9, 255]);
    }
}

#[test]
fn indexed_missing_palette_entry_and_unsupported_compression_are_rejected() {
    let mut input = dib(40, 1, 1, 8, 0, &[1, 2, 3, 0, 1, 0, 0, 0]);
    set(&mut input, 32, 1);
    assert!(decode(&input).is_err());
    for compression in [1, 2, 3, 6] {
        set(&mut input, 16, compression);
        assert!(decode(&input).is_err());
    }
    let mut oversized = dib(40, 1, 1, 1, 0, &[0; 16]);
    set(&mut oversized, 32, 3);
    assert!(decode(&oversized).is_err());
}

#[test]
fn cancellation_before_or_between_rows_never_returns_partial_image() {
    let input = dib(40, 1, 3, 24, 0, &[0; 12]);
    assert_eq!(
        decode_with_cancel(&input, || true).err().unwrap().code(),
        E_ABORT
    );
    let checks = std::cell::Cell::new(0);
    let result = decode_with_cancel(&input, || {
        checks.set(checks.get() + 1);
        checks.get() >= 3
    });
    assert_eq!(result.err().unwrap().code(), E_ABORT);
    assert_eq!(checks.get(), 3);
}
fn set(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn dib(
    header: usize,
    width: i32,
    height: i32,
    bits: u16,
    compression: u32,
    body: &[u8],
) -> Vec<u8> {
    let mut bytes = vec![0; header];
    set(&mut bytes, 0, header as u32);
    set(&mut bytes, 4, width as u32);
    set(&mut bytes, 8, height as u32);
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&bits.to_le_bytes());
    set(&mut bytes, 16, compression);
    bytes.extend_from_slice(body);
    bytes
}
#[test]
fn rgb24_padding_bottom_up_and_top_down_produce_identical_pixels() {
    let bottom = dib(40, 1, 2, 24, 0, &[0, 255, 0, 99, 255, 0, 0, 88]);
    let top = dib(40, 1, -2, 24, 0, &[255, 0, 0, 77, 0, 255, 0, 66]);
    assert_eq!(
        &decode(&bottom).unwrap().pixels[..],
        &[255, 0, 0, 255, 0, 255, 0, 255]
    );
    assert_eq!(
        decode(&bottom).unwrap().pixels,
        decode(&top).unwrap().pixels
    );
}
#[test]
fn rgb32_reserved_byte_is_not_alpha_but_v5_alpha_is_preserved() {
    let opaque = dib(40, 1, -1, 32, 0, &[20, 40, 80, 0]);
    assert_eq!(&decode(&opaque).unwrap().pixels[..], &[20, 40, 80, 255]);
    let mut alpha = dib(124, 1, -1, 32, 3, &[20, 40, 80, 128]);
    for (offset, mask) in [(40, 0xff0000), (44, 0xff00), (48, 0xff), (52, 0xff000000)] {
        set(&mut alpha, offset, mask);
    }
    assert_eq!(&decode(&alpha).unwrap().pixels[..], &[10, 20, 40, 128]);
}
#[test]
fn rgb555_and_external_rgb565_masks_are_normalized() {
    let rgb555 = dib(40, 1, 1, 16, 0, &[0, 0x7c, 0, 0]);
    assert_eq!(&decode(&rgb555).unwrap().pixels[..], &[0, 0, 255, 255]);
    let mut body = Vec::new();
    for mask in [0xf800u32, 0x07e0, 0x001f] {
        body.extend_from_slice(&mask.to_le_bytes());
    }
    body.extend_from_slice(&[0xe0, 0x07, 0, 0]);
    assert_eq!(
        &decode(&dib(40, 1, 1, 16, 3, &body)).unwrap().pixels[..],
        &[0, 255, 0, 255]
    );
}
#[test]
fn malformed_headers_masks_profiles_and_truncation_are_rejected_before_output() {
    let valid = dib(40, 1, 1, 24, 0, &[0; 4]);
    for size in 0..valid.len() {
        assert!(decode(&valid[..size]).is_err());
    }
    for (offset, value) in [
        (0, 0),
        (4, u32::MAX),
        (8, i32::MIN as u32),
        (8, 0),
        (16, 1),
        (32, u32::MAX),
    ] {
        let mut bad = valid.clone();
        set(&mut bad, offset, value);
        assert!(decode(&bad).is_err());
    }
    let mut bad = dib(124, 1, 1, 32, 3, &[0; 4]);
    for (offset, mask) in [(40, 0xff0000), (44, 0xff00), (48, 0xff), (52, 0xff000000)] {
        set(&mut bad, offset, mask);
    }
    let original = bad.clone();
    for (offset, value) in [
        (40, 0xff00),
        (40, 0x550000),
        (40, 0),
        (56, 0x4c494e4b),
        (60, 1),
        (112, 124),
        (116, 32),
    ] {
        let mut bad = original.clone();
        set(&mut bad, offset, value);
        assert!(decode(&bad).is_err());
    }
}
