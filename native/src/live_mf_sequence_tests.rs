//! 显式合成序列门禁；仅测试 fixture 使用 AUD 切帧，不将它作为生产网络解析器。
use super::DecoderOwner;
use std::{
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

fn load(root: &Path, name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    std::fs::File::open(root.join(name))
        .unwrap()
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 4 * 1024 * 1024, "fixture budget");
    bytes
}
fn units(bytes: &[u8]) -> Vec<&[u8]> {
    let offsets: Vec<_> = bytes
        .windows(5)
        .enumerate()
        .filter_map(|(at, v)| (v == [0, 0, 0, 1, 9]).then_some(at))
        .collect();
    assert_eq!(offsets.first(), Some(&0), "fixture must start with AUD");
    assert_eq!(offsets.len(), 24);
    offsets
        .iter()
        .enumerate()
        .map(|(i, &at)| &bytes[at..offsets.get(i + 1).copied().unwrap_or(bytes.len())])
        .collect()
}
fn idr(bytes: &[u8]) -> bool {
    bytes
        .windows(4)
        .any(|v| v[..3] == [0, 0, 1] && v[3] & 31 == 5)
}
#[test]
#[ignore = "requires OTTER_NATIVE_H264_SEQUENCES containing synthetic 24-frame AUD sequences"]
fn actual_continuous_delta_and_idr_restart_preserve_pixels_and_timeline() {
    let root = std::env::var("OTTER_NATIVE_H264_SEQUENCES")
        .expect("explicit synthetic sequence directory");
    for (name, width, height) in [("gray64.h264", 64, 64), ("gray680.h264", 680, 430)] {
        let bytes = load(Path::new(&root), name);
        let frames = units(&bytes);
        for (index, frame) in frames.iter().enumerate() {
            assert_eq!(idr(frame), index % 12 == 0);
        }
        for first in [0, 12] {
            let mut decoder = DecoderOwner::create()
                .unwrap()
                .configure(width, height)
                .unwrap()
                .into_video();
            let stop = AtomicBool::new(false);
            let mut decoded = 0;
            let mut maximum_error = 0;
            for (index, frame) in frames.iter().enumerate().skip(first) {
                let output = decoder
                    .decode(frame, index as i64 + 1, idr(frame), &stop)
                    .unwrap()
                    .expect("sequence must emit without accumulating backlog");
                assert_eq!(
                    (output.timestamp, output.width, output.height),
                    (index as i64 + 1, width, height)
                );
                assert_eq!(output.pixels.len(), width as usize * height as usize * 4);
                // 输入 Y 每帧递增四级，检查内容身份，不能只验证恒定画面和时间戳。
                let expected = ((298 * (32 + index as i32 * 4 - 16) + 128) >> 8) as u8;
                for p in output.pixels.chunks_exact(4) {
                    assert_eq!(p[3], 255);
                    for channel in &p[..3] {
                        maximum_error = maximum_error.max(channel.abs_diff(expected));
                    }
                }
                decoded += 1;
            }
            assert_eq!(decoded, 24 - first);
            assert!(
                maximum_error <= 3,
                "decoded frame content mismatch: {maximum_error}"
            );
            stop.store(true, Ordering::Release);
            assert!(decoder.decode(frames[0], 25, true, &stop).is_err());
            stop.store(false, Ordering::Release);
            assert!(decoder.decode(frames[0], 26, true, &stop).is_err());
            // 失败立即归还名额，不依赖旧 VideoDecoder 的 Drop。
            drop(DecoderOwner::create().unwrap());
            eprintln!("native-mf-sequence: {width}x{height} start={first} decoded={decoded} gray-error={maximum_error} timestamp=true cancel-released=true");
        }
        let mut decoder = DecoderOwner::create()
            .unwrap()
            .configure(width, height)
            .unwrap()
            .into_video();
        assert!(decoder
            .decode(frames[1], 2, false, &AtomicBool::new(false))
            .is_err());
        drop(DecoderOwner::create().unwrap());
    }
}
