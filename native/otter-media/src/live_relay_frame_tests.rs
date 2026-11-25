use super::*;
#[test]
fn header_admission_rejects_oversized_payload_before_body_allocation() {
    let mut header = metadata().encode_header(8).unwrap();
    assert_eq!(decode_header(&header), Ok((metadata(), 8)));
    header[52..56].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(decode_header(&header), Err(Error::Payload));
    assert!(decode_header(&header[..63]).is_err());
    assert!(decode_header(&[0; 65]).is_err());
}
fn metadata() -> Metadata {
    Metadata {
        epoch: 2,
        frame_id: 9,
        capture_timestamp_ms: 100,
        encode_timestamp_ms: 110,
        width: 2,
        height: 1,
        dropped_frames: 3,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
}
#[test]
fn main_golden_header_is_exact_and_payload_is_borrowed() {
    // main live_relay_protocol_tests 的 2x1 BGRA 用例，按既有协议固定字节而非自循环推导。
    let golden: [u8; 64] = [
        78, 76, 76, 86, 1, 1, 0, 64, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0,
        0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 110, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 8, 1, 1,
        0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(metadata().encode_header(8).unwrap(), golden);
    let bytes = [golden.as_slice(), &[1; 8]].concat();
    let decoded = decode_envelope(&bytes).unwrap();
    assert_eq!(decoded.metadata, metadata());
    assert_eq!(decoded.payload, &[1; 8]);
    assert_eq!(decoded.payload.as_ptr(), bytes[64..].as_ptr());
}
#[test]
fn truncation_reserved_fields_lengths_and_unknown_tags_are_rejected() {
    let bytes = [metadata().encode_header(8).unwrap().as_slice(), &[1; 8]].concat();
    for length in 0..bytes.len() {
        assert!(decode_envelope(&bytes[..length]).is_err());
    }
    for offset in [0, 4, 5, 6, 7, 52, 55, 56, 57, 58, 59, 60, 61, 62, 63] {
        let mut invalid = bytes.clone();
        invalid[offset] = 255;
        assert!(decode_envelope(&invalid).is_err(), "offset={offset}");
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_envelope(&trailing).is_err());
}
#[test]
fn identities_timestamps_and_profiles_match_main_bounds_without_allocations() {
    let base = metadata();
    for invalid in [
        Metadata { epoch: 0, ..base },
        Metadata {
            frame_id: 0,
            ..base
        },
        Metadata {
            encode_timestamp_ms: 99,
            ..base
        },
        Metadata { width: 0, ..base },
        Metadata {
            height: 16385,
            ..base
        },
    ] {
        assert!(invalid.validate(8).is_err());
    }
    for size in [0, 4, 9, MAX_FRAME_BYTES + 1, usize::MAX] {
        assert!(base.encode_header(size).is_err());
    }
    let jpeg = Metadata {
        codec: Codec::Jpeg,
        width: 4096,
        height: 4096,
        ..base
    };
    assert!(jpeg.validate(MAX_JPEG_BYTES).is_ok());
    assert!(jpeg.validate(MAX_JPEG_BYTES + 1).is_err());
    assert!(Metadata {
        width: 4097,
        ..jpeg
    }
    .validate(1)
    .is_err());
    assert!(Metadata {
        keyframe: false,
        ..jpeg
    }
    .validate(1)
    .is_err());
    assert!(Metadata {
        color_space: ColorSpace::Hdr10,
        ..jpeg
    }
    .validate(1)
    .is_err());
    let video = Metadata {
        codec: Codec::H264,
        width: 3840,
        height: 2160,
        ..base
    };
    assert!(video.validate(1024 * 1024).is_ok());
    assert!(video.validate(1024 * 1024 + 1).is_err());
    for (width, height) in [(1, 2), (3, 2), (4096, 4096), (4098, 2)] {
        assert!(Metadata {
            width,
            height,
            ..video
        }
        .validate(1)
        .is_err());
    }
}
