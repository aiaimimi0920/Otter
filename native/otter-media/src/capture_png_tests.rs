use super::*;
use crate::{byte_budget::Budget, capture_color::ScRgb};
use std::io::Cursor;
fn frame() -> HdrFrame {
    let data: Vec<_> = [0x4a40u16, 0x3c00, 0, 0x3c00]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    ScRgb::new(&data, 1, 1, 8)
        .unwrap()
        .to_hdr(0.01, 1000.0, 1.0, &AtomicBool::new(false))
        .unwrap()
}
#[test]
fn hdr_png_roundtrips_rgb16_and_writes_pq_metadata_before_image_data() {
    let frame = frame();
    let mut bytes = Vec::new();
    write(&mut bytes, &frame, &AtomicBool::new(false)).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut offset = 8;
    let mut names = Vec::new();
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let name = &bytes[offset + 4..offset + 8];
        let payload = &bytes[offset + 8..offset + 8 + size];
        names.push(name);
        match name {
            b"cICP" => assert_eq!(payload, &[9, 16, 0, 1]),
            b"mDCV" => {
                assert_eq!(size, 24);
                assert_eq!(&payload[..4], &[0x8a, 0x48, 0x39, 0x08]);
                assert_eq!(&payload[16..20], &10_000_000u32.to_be_bytes());
                assert_eq!(&payload[20..], &100u32.to_be_bytes());
            }
            b"cLLI" => {
                assert_eq!(&payload[..4], &scaled(frame.analysis.maximum_nits));
                assert_eq!(&payload[4..], &scaled(frame.analysis.average_nits));
            }
            _ => (),
        }
        offset += size + 12;
    }
    let idat = names.iter().position(|n| *n == b"IDAT").unwrap();
    for name in [b"cHRM", b"cICP", b"mDCV", b"cLLI"] {
        assert_eq!(names.iter().filter(|n| **n == name).count(), 1);
        assert!(names.iter().position(|n| *n == name).unwrap() < idat);
    }
    assert_eq!(*names.last().unwrap(), b"IEND");
    let mut reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
    let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut decoded).unwrap();
    assert_eq!(
        (info.width, info.height, info.bit_depth, info.color_type),
        (1, 1, png::BitDepth::Sixteen, png::ColorType::Rgb)
    );
    assert_eq!(&decoded[..info.buffer_size()], &*frame.rgb16_be);
}
#[test]
fn invalid_input_cancel_and_budget_failure_do_not_touch_destination() {
    let mut frame = frame();
    let cancel = AtomicBool::new(false);
    let mut bytes = Vec::new();
    frame.width = 2;
    assert!(matches!(
        write(&mut bytes, &frame, &cancel),
        Err(Error::Layout)
    ));
    frame.width = 1;
    frame.analysis.maximum_nits = f32::NAN;
    assert!(matches!(
        write(&mut bytes, &frame, &cancel),
        Err(Error::Metadata)
    ));
    frame.analysis.maximum_nits = 1000.0;
    assert!(matches!(
        encode(&mut bytes, &frame, &cancel, |_| None),
        Err(Error::Budget)
    ));
    cancel.store(true, Ordering::Release);
    assert!(matches!(
        write(&mut bytes, &frame, &cancel),
        Err(Error::Cancelled)
    ));
    assert!(bytes.is_empty());
}
struct Failing<'a> {
    calls: usize,
    flush_error: bool,
    cancel: Option<&'a AtomicBool>,
}
impl Write for Failing<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if let Some(cancel) = self.cancel {
            cancel.store(true, Ordering::Release);
        }
        if !self.flush_error && self.cancel.is_none() {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "injected failure",
            ))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("injected flush failure"))
    }
}
#[test]
fn io_failure_and_cancellation_latch_sink_and_release_working_budget() {
    let frame = frame();
    let cancel = AtomicBool::new(false);
    let budget = Budget::new(5 * 1024 * 1024);
    let mut writer = Failing {
        calls: 0,
        flush_error: false,
        cancel: None,
    };
    assert!(encode(&mut writer, &frame, &cancel, |n| budget.reserve(n)).is_err());
    assert_eq!(writer.calls, 1);
    assert_eq!(budget.used(), 0);
    writer.calls = 0;
    writer.cancel = Some(&cancel);
    assert!(matches!(
        encode(&mut writer, &frame, &cancel, |n| budget.reserve(n)),
        Err(Error::Cancelled)
    ));
    assert_eq!(writer.calls, 1);
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    writer.cancel = None;
    writer.flush_error = true;
    assert!(write(&mut writer, &frame, &cancel).is_err());
    let mut bytes = Vec::new();
    encode(&mut bytes, &frame, &cancel, |n| budget.reserve(n)).unwrap();
    assert_eq!(budget.used(), 0);
}
