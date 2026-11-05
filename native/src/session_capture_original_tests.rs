use super::*;
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(
            format!("otter-original-restore-test-{:?}", unsafe {
                CoCreateGuid().unwrap()
            }),
        );
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn count(&self) -> usize {
        std::fs::read_dir(&self.0).unwrap().count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir(&self.0).unwrap();
    }
}
fn png() -> Vec<u8> {
    let cancel = AtomicBool::new(false);
    let bytes: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let frame = crate::capture_color::ScRgb::new(&bytes, 1, 1, 8)
        .unwrap()
        .to_hdr(0.01, 1000.0, 1.0, &cancel)
        .unwrap();
    let mut output = Vec::new();
    crate::capture_png::write(&mut output, &frame, &cancel).unwrap();
    output
}
#[test]
fn restored_original_retains_exact_bytes_and_file_is_deleted_after_last_owner() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let bytes = png();
    let encoded = crate::loom_base64::encode_png_data_url(&bytes, &cancel).unwrap();
    let file = restore_in(encoded.as_str(), &directory.0, &cancel).unwrap();
    assert_eq!(directory.count(), 1);
    assert_eq!(
        crate::capture_file::encode_inline(&file, &cancel)
            .unwrap()
            .as_str(),
        encoded.as_str()
    );
    let other = file.clone();
    drop(file);
    assert_eq!(directory.count(), 1);
    drop(other);
    assert_eq!(directory.count(), 0);
}
#[test]
fn invalid_crc_truncation_trailing_bytes_and_sdr_do_not_leave_files() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let valid = png();
    let mut corrupt = valid.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    let mut trailing = valid.clone();
    trailing.extend_from_slice(b"extra");
    let mut sdr = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut sdr, 1, 1);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[1, 2, 3]).unwrap();
        writer.finish().unwrap();
    }
    for bytes in [corrupt, trailing, valid[..valid.len() - 6].to_vec(), sdr] {
        let encoded = crate::loom_base64::encode_png_data_url(&bytes, &cancel).unwrap();
        assert!(restore_in(encoded.as_str(), &directory.0, &cancel).is_err());
        assert_eq!(directory.count(), 0);
    }
    for reference in [
        "file:///C:/image.png",
        "https://example.invalid/a.png",
        "data:image/png;base64,invalid",
        "data:image/png;base64,",
    ] {
        assert!(restore_in(reference, &directory.0, &cancel).is_err());
        assert_eq!(directory.count(), 0);
    }
    cancel.store(true, Ordering::Release);
    assert!(restore_in("invalid", &directory.0, &cancel).is_err());
    assert_eq!(directory.count(), 0);
}
