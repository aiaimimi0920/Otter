//! Signed invitation -> real PNG -> WIC -> QR reader round trips, budgets, and close cancellation without reading actual user files.
use super::*;
fn directory() -> PathBuf {
    let root =
        PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-account-qr-tests");
    let path = root.join(format!("{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn write_png(code: &Code, scale: usize, path: &std::path::Path) {
    let side = (code.size + 8) * scale;
    let mut pixels = vec![255u8; side * side];
    for y in 0..code.size {
        for x in 0..code.size {
            if code.colors[y * code.size + x] == qrcode::Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        pixels[((y + 4) * scale + dy) * side + (x + 4) * scale + dx] = 0;
                    }
                }
            }
        }
    }
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, side as u32, side as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
}
#[test]
fn qr_dense_export_and_account_json_or_link_roundtrip() {
    let directory = directory();
    let scope = serde_json::from_value(serde_json::json!("a".repeat(64))).unwrap();
    let (content, _) = crate::projection_account::tests::scene_tests::fixture(scope, "receiver-1");
    let json = serde_json::to_string(&content.envelope).unwrap();
    let link = content.envelope.link().unwrap();
    for (index, value) in [&json, &link].into_iter().enumerate() {
        for scale in [3, 6] {
            let path = directory.join(format!("dense-{index}-{scale}.png"));
            write_png(&encode_bytes(value.as_bytes()).unwrap(), scale, &path);
            assert_eq!(
                &decode(path.clone(), &AtomicBool::new(false)).unwrap(),
                value
            );
            assert_eq!(
                decode(path, &AtomicBool::new(true)),
                Err(crate::ui_text::text("share-qr-cancelled"))
            );
        }
    }
    let invalid = json.replace("account-1", "account-2");
    let path = directory.join("tampered-v2.png");
    write_png(&encode_bytes(invalid.as_bytes()).unwrap(), 3, &path);
    assert_eq!(
        decode(path, &AtomicBool::new(false)),
        Err(crate::ui_text::text("share-invalid-invitation"))
    );
}
#[test]
fn qr_decoder_rejects_non_png_oversize_and_invalid_dimension_before_decode() {
    let directory = directory();
    let path = directory.join("invalid.png");
    for bytes in [b"not a PNG".to_vec(), vec![0; 4 * 1024 * 1024 + 1]] {
        std::fs::write(&path, bytes).unwrap();
        assert!(decode(path.clone(), &AtomicBool::new(false)).is_err());
    }
    let mut header = vec![0; 33];
    header[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    header[12..16].copy_from_slice(b"IHDR");
    header[16..20].copy_from_slice(&4097u32.to_be_bytes());
    header[20..24].copy_from_slice(&1u32.to_be_bytes());
    std::fs::write(&path, header).unwrap();
    assert_eq!(
        decode(path, &AtomicBool::new(false)),
        Err(crate::ui_text::text("share-qr-dimensions"))
    );
}
