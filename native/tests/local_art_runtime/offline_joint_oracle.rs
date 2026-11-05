//! 独立条带像素oracle；不调用产品合成器，不从实际输出生成预期。
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

pub(super) fn expected(stage: u64) -> (u32, u32, Vec<u8>) {
    assert!((1..=3).contains(&stage));
    let (w, h) = match stage {
        1 => (16, 8),
        2 => (32, 16),
        _ => (320, 160),
    };
    let pixels = (0..h)
        .flat_map(|_| {
            (0..w).flat_map(|x| {
                if stage == 3 {
                    // 32→320的原恢复底图在flip后成为权威owner；独立有理数双线性条带。
                    let red = (329_i32 - 2 * x as i32).clamp(0, 20) as u32;
                    return [
                        ((255 * red + 10) / 20) as u8,
                        0,
                        ((255 * (20 - red) + 10) / 20) as u8,
                        255,
                    ];
                }
                let source_x = match stage {
                    1 => x + 4,
                    _ => x,
                };
                if source_x < 16 {
                    [0, 0, 255, 255]
                } else {
                    [255, 0, 0, 255]
                }
            })
        })
        .collect();
    (w, h, pixels)
}
pub(super) fn png() -> Vec<u8> {
    let (w, h, pixels) = expected(2);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    bytes
}
pub(super) fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    assert!(bytes.len() <= 64 * 1024);
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert!(reader.info().width <= 320 && reader.info().height <= 160);
    let mut out = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut out).unwrap();
    out.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => out,
        png::ColorType::Rgb => out
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("unexpected ordinary product PNG type {other:?}"),
    };
    (info.width, info.height, pixels)
}
pub(super) fn row(row: &Value, stage: u64) {
    let bytes = STANDARD
        .decode(
            row["src"]
                .as_str()
                .unwrap()
                .strip_prefix("data:image/png;base64,")
                .unwrap(),
        )
        .unwrap();
    assert_eq!(decode(&bytes), expected(stage));
}
#[test]
fn ordinary_offline_pixel_oracle_has_independent_crop_and_flipped_bands() {
    let (w, h, p) = expected(1);
    assert_eq!((w, h), (16, 8));
    assert_eq!(&p[..48], [0, 0, 255, 255].repeat(12));
    assert_eq!(&p[48..64], [255, 0, 0, 255].repeat(4));
    let (w, h, p) = expected(3);
    assert_eq!((w, h), (320, 160));
    assert_eq!(&p[..155 * 4], [255, 0, 0, 255].repeat(155));
    assert_eq!(&p[155 * 4..156 * 4], [242, 0, 13, 255]);
    assert_eq!(&p[159 * 4..161 * 4], [140, 0, 115, 255, 115, 0, 140, 255]);
    assert_eq!(&p[164 * 4..165 * 4], [13, 0, 242, 255]);
    assert_eq!(&p[165 * 4..320 * 4], [0, 0, 255, 255].repeat(155));
    assert_eq!(decode(&png()), expected(2));
}
