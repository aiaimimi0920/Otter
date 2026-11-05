//! 固定快速滤波仍为逐像素无损PNG，不改变alpha、尺寸或输出预算。
use super::*;
fn decode(size: [u32; 2], bgra: Vec<u8>) -> Vec<u8> {
    let encoded = encode(size, bgra).unwrap();
    let bytes = STANDARD.decode(encoded).unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!([reader.info().width, reader.info().height], size);
    let mut output = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut output).unwrap();
    output
}
#[test]
fn canvas_fast_png_filter_preserves_native_high_entropy_pixels() {
    let mut seed = 0x1234abcd_u32;
    let mut bgra = Vec::new();
    let mut rgba = Vec::new();
    for _ in 0..512 * 512 {
        let mut pixel = [0u8; 4];
        for channel in &mut pixel[..3] {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *channel = (seed >> 24) as u8;
        }
        pixel[3] = 255;
        bgra.extend_from_slice(&pixel);
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    assert_eq!(decode([512, 512], bgra), rgba);
}
#[test]
fn canvas_fast_png_filter_preserves_all_alpha_values_and_transparent_clearing() {
    let mut bgra = Vec::new();
    let mut rgba = Vec::new();
    for alpha in 0..=255u8 {
        let pixel = [alpha / 3, alpha / 2, alpha, alpha];
        bgra.extend_from_slice(&pixel);
        // 独立浮点oracle，不调用生产的整数反预乘或新编码函数产生预期。
        for value in [pixel[2], pixel[1], pixel[0]] {
            rgba.push(if alpha == 0 {
                0
            } else {
                (f64::from(value) * 255.0 / f64::from(alpha)).round() as u8
            });
        }
        rgba.push(alpha);
    }
    bgra.extend_from_slice(&[99, 88, 77, 0]);
    rgba.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(decode([257, 1], bgra), rgba);
    assert!(encode([1, 1], vec![0; 3]).is_err());
}
