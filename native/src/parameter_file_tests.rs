//! 有界流编码合同；全部用合成字节，不读取用户文件。
use super::*;
use std::io::Cursor;

struct ShortRead<'a> {
    input: Cursor<Vec<u8>>,
    cancel: Option<&'a AtomicBool>,
}
impl Read for ShortRead<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some(cancel) = self.cancel {
            cancel.store(true, Ordering::Release);
        }
        let length = buffer.len().min(5);
        self.input.read(&mut buffer[..length])
    }
}

#[test]
fn arbitrary_short_reads_preserve_bytes_and_single_padding() {
    let cancel = AtomicBool::new(false);
    for length in [0, 1, 2, 3, 5, 12287, 12288, 12289, 65537] {
        let bytes: Vec<_> = (0..length).map(|index| index as u8).collect();
        let expected = format!("data:image/png;base64,{}", STANDARD.encode(&bytes));
        let input = ShortRead {
            input: Cursor::new(bytes),
            cancel: None,
        };
        assert_eq!(
            encode(input, "image/png", &cancel, expected.len()).unwrap(),
            expected
        );
    }
}

#[test]
fn budget_checks_include_header_and_trailing_bytes() {
    let cancel = AtomicBool::new(false);
    let expected = "data:image/png;base64,AA==";
    assert_eq!(
        encode(Cursor::new([0]), "image/png", &cancel, expected.len()).unwrap(),
        expected
    );
    assert!(matches!(
        encode(Cursor::new([0]), "image/png", &cancel, expected.len() - 1),
        Err(Error::Capacity)
    ));
    assert!(matches!(
        encode(Cursor::new([]), "image/png", &cancel, 1),
        Err(Error::Capacity)
    ));
}

#[test]
fn cancellation_before_and_during_read_discards_result() {
    let cancel = AtomicBool::new(true);
    assert!(matches!(
        encode(Cursor::new([1]), "image/png", &cancel, 100),
        Err(Error::Cancelled)
    ));
    cancel.store(false, Ordering::Release);
    let input = ShortRead {
        input: Cursor::new(vec![1, 2, 3]),
        cancel: Some(&cancel),
    };
    assert!(matches!(
        encode(input, "image/png", &cancel, 100),
        Err(Error::Cancelled)
    ));
}

#[test]
fn extension_mapping_is_case_insensitive_and_unknown_is_binary() {
    assert_eq!(mime(Path::new("photo.PNG")), "image/png");
    assert_eq!(
        mime(Path::new("folder.png/no-extension")),
        "application/octet-stream"
    );
    assert_eq!(mime(Path::new("value.unknown")), "application/octet-stream");
    assert_eq!(mime(Path::new("data.json")), "application/json");
}

#[test]
fn io_errors_do_not_return_partial_data() {
    struct Failed;
    impl Read for Failed {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("synthetic read failure"))
        }
    }
    assert!(matches!(
        encode(Failed, "image/png", &AtomicBool::new(false), 100),
        Err(Error::Io(_))
    ));
}
