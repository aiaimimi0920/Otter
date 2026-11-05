use super::*;
use crate::capture_color::ScRgb;
use std::{fs, io::Cursor, sync::atomic::Ordering};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root =
            Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-hdr-file-tests");
        fs::create_dir_all(root).unwrap();
        let path = root.join(format!(
            "{:?}",
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn target(&self) -> PathBuf {
        self.0.join("HDR 原图.png")
    }
    fn entries(&self) -> usize {
        fs::read_dir(&self.0).unwrap().count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.target());
        let _ = fs::remove_dir(&self.0);
    }
}
fn frame() -> HdrFrame {
    let data: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    ScRgb::new(&data, 1, 1, 8)
        .unwrap()
        .to_hdr(0.01, 1000.0, 1.0, &AtomicBool::new(false))
        .unwrap()
}
#[test]
fn original_copy_preserves_png_bytes_without_publishing_until_commit() {
    let source = Fixture::new();
    let target = Fixture::new();
    let cancel = AtomicBool::new(false);
    PreparedHdr::prepare(&source.target(), &frame(), &cancel)
        .unwrap()
        .commit_new(&cancel)
        .unwrap();
    let file = std::fs::File::open(source.target()).unwrap();
    let expected = fs::read(source.target()).unwrap();
    let staged = prepare_original(&file, &target.target(), &cancel).unwrap();
    assert!(!target.target().exists());
    staged.commit_new(&cancel).unwrap();
    assert_eq!(fs::read(target.target()).unwrap(), expected);
    // 第二次从相同句柄复制不能依赖前一次留下的游标；冲突目标仍不覆盖。
    let staged = prepare_original(&file, &target.target(), &cancel).unwrap();
    assert!(staged.commit_new(&cancel).is_err());
    assert_eq!(fs::read(target.target()).unwrap(), expected);
    assert_eq!(target.entries(), 1);
}
#[test]
fn original_copy_rejects_invalid_format_extension_and_cancel_without_final_files() {
    let source = Fixture::new();
    let target = Fixture::new();
    let cancel = AtomicBool::new(false);
    fs::write(source.target(), b"not a PNG source").unwrap();
    let file = std::fs::File::open(source.target()).unwrap();
    assert!(prepare_original(&file, &target.target(), &cancel).is_err());
    assert_eq!(target.entries(), 0);
    drop(file);
    // 使用另一个 fixture，避免为测试覆盖已有文件。
    let valid = Fixture::new();
    PreparedHdr::prepare(&valid.target(), &frame(), &cancel)
        .unwrap()
        .commit_new(&cancel)
        .unwrap();
    let file = std::fs::File::open(valid.target()).unwrap();
    assert!(prepare_original(&file, &target.0.join("wrong.jpg"), &cancel).is_err());
    let staged = prepare_original(&file, &target.target(), &cancel).unwrap();
    cancel.store(true, Ordering::Release);
    assert!(staged.commit_new(&cancel).is_err());
    assert!(prepare_original(&file, &target.target(), &cancel).is_err());
    assert_eq!(target.entries(), 0);
}
#[test]
fn hdr_file_is_hidden_until_commit_then_roundtrips_full_rgb16() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let frame = frame();
    let prepared = PreparedHdr::prepare(&fixture.target(), &frame, &cancel).unwrap();
    assert!(!fixture.target().exists());
    assert_eq!(fixture.entries(), 1);
    let path = prepared.commit_new(&cancel).unwrap();
    assert!(path.exists());
    assert_eq!(fixture.entries(), 1);
    let data = fs::read(&path).unwrap();
    let mut reader = png::Decoder::new(Cursor::new(data)).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.bit_depth, png::BitDepth::Sixteen);
    assert_eq!(&pixels[..info.buffer_size()], &*frame.rgb16_be);
    cancel.store(true, Ordering::Release);
    assert!(path.exists());
}
#[test]
fn collision_cancel_and_drop_preserve_existing_files_and_remove_only_owned_staging() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let frame = frame();
    fs::write(fixture.target(), b"existing user content").unwrap();
    let staged = PreparedHdr::prepare(&fixture.target(), &frame, &cancel).unwrap();
    assert!(staged.commit_new(&cancel).is_err());
    assert_eq!(
        fs::read(fixture.target()).unwrap(),
        b"existing user content"
    );
    assert_eq!(fixture.entries(), 1);
    let staged = PreparedHdr::prepare(&fixture.target(), &frame, &cancel).unwrap();
    cancel.store(true, Ordering::Release);
    assert!(staged.commit_new(&cancel).is_err());
    assert_eq!(fixture.entries(), 1);
    cancel.store(false, Ordering::Release);
    drop(PreparedHdr::prepare(&fixture.target(), &frame, &cancel).unwrap());
    assert_eq!(fixture.entries(), 1);
    assert_eq!(
        fs::read(fixture.target()).unwrap(),
        b"existing user content"
    );
}
#[test]
fn invalid_pixels_and_extension_never_publish_or_leave_partial_staging() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut frame = frame();
    assert!(PreparedHdr::prepare(&fixture.0.join("wrong.jpg"), &frame, &cancel).is_err());
    frame.width = 2;
    assert!(PreparedHdr::prepare(&fixture.target(), &frame, &cancel).is_err());
    assert_eq!(fixture.entries(), 0);
    cancel.store(true, Ordering::Release);
    assert!(PreparedHdr::prepare(&fixture.target(), &frame, &cancel).is_err());
    assert_eq!(fixture.entries(), 0);
}
