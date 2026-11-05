//! 内存 WIC 编码夹具；独立位置表验证 EXIF 八方向，不读取用户照片。
use super::*;
use windows::core::w;
use windows::Win32::{
    Foundation::HGLOBAL,
    System::{Com::StructuredStorage::*, Variant::VT_UI2},
};
const COLORS: [[u8; 3]; 6] = [
    [0, 0, 255],
    [0, 255, 0],
    [255, 0, 0],
    [0, 255, 255],
    [255, 0, 255],
    [255, 255, 0],
];
const ORDER: [[usize; 6]; 8] = [
    [0, 1, 2, 3, 4, 5],
    [2, 1, 0, 5, 4, 3],
    [5, 4, 3, 2, 1, 0],
    [3, 4, 5, 0, 1, 2],
    [0, 3, 1, 4, 2, 5],
    [3, 0, 4, 1, 5, 2],
    [5, 2, 4, 1, 3, 0],
    [2, 5, 1, 4, 0, 3],
];
fn encoded(tiff: bool, orientation: u16) -> Vec<u8> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true).unwrap();
        let encoder = factory
            .CreateEncoder(
                if tiff {
                    &GUID_ContainerFormatTiff
                } else {
                    &GUID_ContainerFormatJpeg
                },
                std::ptr::null(),
            )
            .unwrap();
        encoder
            .Initialize(&stream, WICBitmapEncoderNoCache)
            .unwrap();
        let mut frame = None;
        encoder
            .CreateNewFrame(&mut frame, std::ptr::null_mut())
            .unwrap();
        let frame = frame.unwrap();
        frame.Initialize(None).unwrap();
        frame.SetSize(48, 32).unwrap();
        let mut format = GUID_WICPixelFormat24bppBGR;
        frame.SetPixelFormat(&mut format).unwrap();
        assert_eq!(format, GUID_WICPixelFormat24bppBGR);
        if tiff {
            let mut value = PROPVARIANT::default();
            (*value.Anonymous.Anonymous).vt = VT_UI2;
            (*value.Anonymous.Anonymous).Anonymous.uiVal = orientation;
            frame
                .GetMetadataQueryWriter()
                .unwrap()
                .SetMetadataByName(w!("/ifd/{ushort=274}"), &value)
                .unwrap();
        }
        let bytes: Vec<u8> = (0..32)
            .flat_map(|y| (0..48).flat_map(move |x| COLORS[(y / 16) * 3 + x / 16]))
            .collect();
        frame.WritePixels(32, 48 * 3, &bytes).unwrap();
        frame.Commit().unwrap();
        encoder.Commit().unwrap();
        let mut stat = STATSTG::default();
        stream.Stat(&mut stat, STATFLAG_NONAME).unwrap();
        let mut bytes = vec![0; stat.cbSize as usize];
        stream.Seek(0, STREAM_SEEK_SET, None).unwrap();
        let mut read = 0;
        stream
            .Read(
                bytes.as_mut_ptr().cast(),
                bytes.len() as u32,
                Some(&mut read),
            )
            .ok()
            .unwrap();
        assert_eq!(read as usize, bytes.len());
        if tiff {
            bytes
        } else {
            jpeg_exif(&bytes, orientation, 3)
        }
    }
}
fn jpeg_exif(jpeg: &[u8], orientation: u16, kind: u16) -> Vec<u8> {
    let mut exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01".to_vec();
    exif.extend_from_slice(&kind.to_le_bytes());
    exif.extend_from_slice(&1u32.to_le_bytes());
    exif.extend_from_slice(&orientation.to_le_bytes());
    exif.extend_from_slice(&[0; 6]);
    let mut bytes = vec![0xff, 0xd8, 0xff, 0xe1];
    bytes.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    bytes.extend(exif);
    bytes.extend_from_slice(&jpeg[2..]);
    bytes
}
fn check(image: &Image, orientation: usize) {
    let (width, height) = if orientation <= 4 { (48, 32) } else { (32, 48) };
    assert_eq!(
        (image.width, image.height),
        (width, height),
        "orientation {orientation}"
    );
    for (cell, expected) in ORDER[orientation - 1].iter().enumerate() {
        let x = (cell as u32 % (width / 16)) * 16 + 8;
        let y = (cell as u32 / (width / 16)) * 16 + 8;
        let pixel = &image.pixels[((y * width + x) * 4) as usize..][..4];
        assert!(
            pixel[..3]
                .iter()
                .zip(COLORS[*expected])
                .all(|(a, b)| a.abs_diff(b) < 12),
            "orientation {orientation}, cell {cell}: {pixel:?}"
        );
        assert_eq!(pixel[3], 255);
    }
}
#[test]
fn surface_orientation_jpeg_all_eight_use_oriented_dimensions_and_pixels() {
    let _com = apartment();
    for orientation in 1..=8 {
        let bytes = encoded(false, orientation);
        let image = Image::from_surface(&bytes, "image/jpeg", None, || false).unwrap();
        check(&image, orientation as usize);
    }
}
#[test]
fn surface_orientation_tiff_all_eight_use_oriented_dimensions_and_pixels() {
    let _com = apartment();
    for orientation in 1..=8 {
        let bytes = encoded(true, orientation);
        let image = Image::from_surface(&bytes, "image/tiff", None, || false).unwrap();
        check(&image, orientation as usize);
    }
}

impl Image {
    pub(crate) fn oriented_jpeg_for_test() -> Vec<u8> {
        encoded(false, 6)
    }
}
fn without_exif(bytes: &[u8]) -> Vec<u8> {
    let end = 4 + usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    [bytes[..2].to_vec(), bytes[end..].to_vec()].concat()
}
#[test]
fn surface_orientation_missing_invalid_and_wrong_type_are_identity() {
    let _com = apartment();
    let plain = without_exif(&encoded(false, 6));
    let cases = [
        plain.clone(),
        jpeg_exif(&plain, 0, 3),
        jpeg_exif(&plain, 9, 3),
        jpeg_exif(&plain, 6, 4),
    ];
    for bytes in cases {
        check(
            &Image::from_surface(&bytes, "image/jpeg", None, || false).unwrap(),
            1,
        );
    }
    for direction in [0, 9] {
        check(
            &Image::from_surface(&encoded(true, direction), "image/tiff", None, || false).unwrap(),
            1,
        );
    }
}
#[test]
fn surface_orientation_does_not_change_ordinary_or_relay_decode() {
    let _com = apartment();
    let bytes = encoded(false, 6);
    check(&Image::from_encoded(&bytes).unwrap(), 1);
    check(&Image::from_relay_jpeg(&bytes, 48, 32).unwrap(), 1);
    assert!(Image::from_relay_jpeg(&bytes, 32, 48).is_err());
}
#[test]
fn surface_orientation_cancellation_at_each_boundary_then_success() {
    let _com = apartment();
    let bytes = encoded(false, 5);
    for boundary in 1..=8 {
        let calls = std::cell::Cell::new(0);
        let image = Image::from_surface(&bytes, "image/jpeg", None, || {
            calls.set(calls.get() + 1);
            calls.get() >= boundary
        });
        assert!(image.is_err(), "boundary {boundary}");
    }
    check(
        &Image::from_surface(&bytes, "image/jpeg", None, || false).unwrap(),
        5,
    );
}
#[test]
fn surface_orientation_oversized_raw_frame_rejected_before_metadata() {
    let _com = apartment();
    let mut bytes = encoded(false, 6);
    let start = bytes.windows(2).position(|b| b == [0xff, 0xc0]).unwrap();
    bytes[start + 5..start + 9].copy_from_slice(&[0x20, 0, 0x20, 0]);
    let calls = std::cell::Cell::new(0);
    let error = Image::from_surface(&bytes, "image/jpeg", None, || {
        calls.set(calls.get() + 1);
        false
    })
    .err()
    .unwrap();
    assert!(error
        .message()
        .contains("Image exceeds native decode budget"));
    assert_eq!(
        calls.get(),
        1,
        "raw size rejection must precede metadata boundary"
    );
}
