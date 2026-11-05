//! Chromium Canvas 8x8 红/绿 WebP 固定夹具；含 sRGB ICC，比较全部像素而非只测尺寸。
use super::*;
use base64::Engine;
// 2026-10-21 Chromium 实际生成，原始脚本和输出在 linshi/otter-v196-browser.*。
const RGB:&str="UklGRjgCAABXRUJQVlA4WAoAAAAgAAAABwAABwAASUNDUMgBAAAAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADZWUDggSgAAAFACAJ0BKggACAABQCYlqAJ0twCjAVf/uAAODAD8e//dG9/91h/7okn/sf59gf+8Cf9H+fYH/a3Uftkhbz/+U/A/2yQt5//KBAAA";
const RGBA:&str="UklGRlICAABXRUJQVlA4WAoAAAAwAAAABwAABwAASUNDUMgBAAAAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADZBTFBIEgAAAAEP8MD/iIgAEESC/2APEf0PAVZQOCBKAAAAUAIAnQEqCAAIAAFAJiWoAnS3AKMBV/+4AA4MAPx7/90b3/3WH/uiSf+x/n2B/7wJ/0f59gf9rdR+2SFvP/5T8D/bJC3n/8oEAAA=";
#[test]
fn surface_webp_lossy_rgb_and_alpha_match_chromium_pixels() {
    let opaque: [[u8; 4]; 8] = [
        [254, 1, 2, 255],
        [254, 1, 2, 255],
        [252, 2, 2, 255],
        [171, 47, 0, 255],
        [83, 210, 19, 255],
        [2, 255, 0, 255],
        [0, 255, 0, 255],
        [0, 255, 0, 255],
    ];
    let transparent: [[u8; 4]; 8] = [
        [253, 0, 2, 128],
        [253, 0, 2, 128],
        [251, 2, 2, 128],
        [169, 46, 0, 128],
        [83, 210, 19, 255],
        [2, 255, 0, 255],
        [0, 255, 0, 255],
        [0, 255, 0, 255],
    ];
    for (source, row) in [(RGB, opaque), (RGBA, transparent)] {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(source)
            .unwrap();
        let image = Image::from_surface(&bytes, "image/webp", None, || false).unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        for (index, pixel) in image.pixels.chunks_exact(4).enumerate() {
            let expected = row[index % 8];
            for (dst, src) in [(0, 2), (1, 1), (2, 0)] {
                let expected =
                    ((u16::from(expected[src]) * u16::from(expected[3]) + 127) / 255) as u8;
                assert!(
                    pixel[dst].abs_diff(expected) <= 2,
                    "pixel {index} channel {dst}: {} != {expected}",
                    pixel[dst]
                );
            }
            assert_eq!(pixel[3], expected[3]);
        }
    }
}
