//! 合成文件/流回归，不访问会话提供的路径或网络资源。
use super::*;
#[test]
fn bounded_read_rejects_growth_truncation_and_cancel_without_partial_result() {
    let cancel = AtomicBool::new(false);
    assert!(read(&mut io::Cursor::new([1, 2]), 3, &cancel).is_err());
    assert!(read(&mut io::Cursor::new([1, 2, 3]), 2, &cancel).is_err());
    assert!(read(&mut io::Cursor::new([]), MAX_BYTES + 1, &cancel).is_err());
    assert!(read(&mut io::Cursor::new([]), 0, &cancel).is_err());
    assert_eq!(
        read(&mut io::Cursor::new([1]), 1, &AtomicBool::new(true))
            .err()
            .unwrap()
            .code(),
        E_ABORT
    );
    let bytes = read(&mut io::Cursor::new([1, 2, 3]), 3, &cancel).unwrap();
    assert_eq!(&*bytes, &[1, 2, 3]);
    let weak = bytes.downgrade();
    drop(bytes);
    assert!(weak.upgrade().is_none());
}
#[test]
fn cancellation_raised_by_read_is_checked_before_next_io_or_decode() {
    struct Cancels<'a>(&'a AtomicBool);
    impl Read for Cancels<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.0.store(true, Ordering::Release);
            buffer[0] = 1;
            Ok(1)
        }
    }
    let cancel = AtomicBool::new(false);
    assert_eq!(
        read(&mut Cancels(&cancel), 2, &cancel)
            .err()
            .unwrap()
            .code(),
        E_ABORT
    );
}
#[test]
fn real_file_decodes_on_worker_without_leaving_handle_or_pixel_owner() {
    use base64::Engine;
    let expected: crate::pixels::Pixels = vec![12, 34, 56, 255].into();
    let encoded = super::super::loom_inline::prepare(
        1,
        1,
        expected.clone(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = encoded.descriptor() else {
        panic!("PNG expected");
    };
    let path =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-session-image-{}.png",
            super::super::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
    std::fs::write(
        &path,
        base64::engine::general_purpose::STANDARD
            .decode(base64)
            .unwrap(),
    )
    .unwrap();
    let file = File::open(&path).unwrap();
    let image = std::thread::spawn(move || load(file, &AtomicBool::new(false)))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!((image.width, image.height), (1, 1));
    assert_eq!(image.pixels, expected);
    let weak = image.pixels.downgrade();
    drop(image);
    assert!(weak.upgrade().is_none());
    std::fs::remove_file(path).unwrap();
}
