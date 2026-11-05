//! 仅创建进程内 DataPackage，不访问 Windows 当前剪贴板或系统历史。
use super::*;
use windows::{
    core::HSTRING,
    ApplicationModel::DataTransfer::DataPackage,
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
};

fn decode(package: &DataPackage, remaining: usize) -> Result<Option<Content>> {
    content::decode(
        &package.GetView()?,
        remaining,
        &AtomicBool::new(false),
        Instant::now() + Duration::from_secs(5),
    )
}
fn bitmap(bytes: &[u8]) -> DataPackage {
    let stream = InMemoryRandomAccessStream::new().unwrap();
    let writer = DataWriter::CreateDataWriter(&stream.GetOutputStreamAt(0).unwrap()).unwrap();
    writer.WriteBytes(bytes).unwrap();
    let cancel = AtomicBool::new(false);
    let deadline = Instant::now() + Duration::from_secs(5);
    let store = || -> Result<()> {
        wait::operation!(writer.StoreAsync()?, &cancel, deadline)?;
        Ok(())
    };
    store().unwrap();
    let reference = RandomAccessStreamReference::CreateFromStream(&stream).unwrap();
    let package = DataPackage::new().unwrap();
    package.SetBitmap(&reference).unwrap();
    package
}
#[test]
fn text_and_unsupported_formats_are_read_only_and_bounded() {
    let _apartment = Apartment::new().unwrap();
    let package = DataPackage::new().unwrap();
    assert!(decode(&package, MAX_TOTAL_BYTES).unwrap().is_none());
    package
        .SetHtmlFormat(&HSTRING::from("<b>not text</b>"))
        .unwrap();
    assert!(decode(&package, MAX_TOTAL_BYTES).unwrap().is_none());
    package
        .SetText(&HSTRING::from("你好 <b>literal</b>\r\nsecond"))
        .unwrap();
    let Some(Content::Text(text)) = decode(&package, MAX_TOTAL_BYTES).unwrap() else {
        panic!("Missing text")
    };
    assert_eq!(text, "你好 <b>literal</b>\r\nsecond");
    for (index, invalid) in [
        " ".into(),
        "x".repeat(MAX_TEXT_BYTES + 1),
        "x\r".repeat(256),
    ]
    .into_iter()
    .enumerate()
    {
        package.SetText(&HSTRING::from(invalid)).unwrap();
        assert!(
            decode(&package, MAX_TOTAL_BYTES).unwrap().is_none(),
            "invalid fixture {index}"
        );
    }
    package
        .SetText(&HSTRING::from("字".repeat(30_000)))
        .unwrap();
    assert!(decode(&package, MAX_TOTAL_BYTES).unwrap().is_none());
    package.SetText(&HSTRING::from("1234")).unwrap();
    assert!(decode(&package, 3).unwrap().is_none());
}

#[test]
fn data_package_truncates_embedded_nul_before_the_reader() {
    let _apartment = Apartment::new().unwrap();
    let package = DataPackage::new().unwrap();
    let input = HSTRING::from("a\0b");
    assert_eq!(input.len(), 3);
    package.SetText(&input).unwrap();
    let view = package.GetView().unwrap();
    let raw = || -> Result<HSTRING> {
        wait::operation!(
            view.GetTextAsync()?,
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(5)
        )
    };
    let value = raw().unwrap();
    assert_eq!(value.len(), 1);
    assert_eq!(value.to_string(), "a");
    let Some(Content::Text(text)) = decode(&package, MAX_TOTAL_BYTES).unwrap() else {
        panic!("Missing normalized text")
    };
    assert_eq!(text, "a");
}

#[test]
fn bitmap_stream_accepts_real_bmp_without_relabeling_it_as_png() {
    let _apartment = Apartment::new().unwrap();
    let mut bytes = vec![0; 58];
    bytes[..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&58u32.to_le_bytes());
    bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&1i32.to_le_bytes());
    bytes[22..26].copy_from_slice(&1i32.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes[54..57].copy_from_slice(&[20, 40, 60]);
    let package = bitmap(&bytes);
    let Some(Content::Bitmap {
        encoded,
        width,
        height,
        thumbnail,
    }) = decode(&package, MAX_TOTAL_BYTES).unwrap()
    else {
        panic!("Missing BMP")
    };
    assert_eq!((width, height), (1, 1));
    assert_eq!(&encoded[..], bytes);
    assert_eq!((thumbnail.width(), thumbnail.height()), (1, 1));
    assert_eq!(thumbnail.pixels(), &[20, 40, 60, 255]);
}
#[test]
fn bitmap_keeps_encoded_bytes_and_has_priority_over_text() {
    let _apartment = Apartment::new().unwrap();
    let png = include_bytes!("../../src-tauri/icons/32x32.png");
    let package = bitmap(png);
    package.SetText(&HSTRING::from("not selected")).unwrap();
    let Some(Content::Bitmap {
        encoded,
        width,
        height,
        thumbnail,
    }) = decode(&package, MAX_TOTAL_BYTES).unwrap()
    else {
        panic!("Missing bitmap")
    };
    assert_eq!((width, height), (32, 32));
    assert_eq!(&encoded[..], png);
    assert_eq!((thumbnail.width(), thumbnail.height()), (32, 32));
    assert!(decode(&package, png.len() - 1).unwrap().is_none());
    let invalid = bitmap(b"not an image");
    invalid
        .SetText(&HSTRING::from("must not fall back"))
        .unwrap();
    assert!(decode(&invalid, MAX_TOTAL_BYTES).is_err());
}
#[test]
fn oversized_encoded_bitmap_is_skipped_before_decode() {
    let _apartment = Apartment::new().unwrap();
    let package = bitmap(&vec![0; MAX_IMAGE_BYTES + 1]);
    assert!(decode(&package, MAX_TOTAL_BYTES).unwrap().is_none());
}
#[test]
fn cancellation_and_expired_deadline_do_not_read_history() {
    let cancel = AtomicBool::new(true);
    assert_eq!(read(&cancel).err().unwrap().code(), E_ABORT);
    assert_eq!(
        check(&AtomicBool::new(false), Instant::now())
            .unwrap_err()
            .code(),
        E_ABORT
    );
}

#[test]
fn read_permit_is_exclusive_and_released_without_reading_system_history() {
    let permit = ReadPermit::acquire().unwrap();
    assert!(read(&AtomicBool::new(false)).is_err());
    drop(permit);
    let _again = ReadPermit::acquire().unwrap();
}

#[test]
fn bitmap_dimensions_and_pixel_bytes_are_rejected_before_pixel_allocation() {
    let _apartment = Apartment::new().unwrap();
    for (width, height, message) in [
        (8193, 1, "Image dimensions exceed input policy"),
        (4097, 4097, "Image exceeds native decode budget"),
    ] {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![0; width as usize * height as usize * 4])
                .unwrap();
            writer.finish().unwrap();
        }
        assert!(bytes.len() < MAX_IMAGE_BYTES);
        let error = crate::image_decode::Image::from_clipboard_history(&bytes)
            .err()
            .unwrap();
        assert!(error.message().contains(message));
    }
}
