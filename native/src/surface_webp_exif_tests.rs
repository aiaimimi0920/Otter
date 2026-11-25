//! Chromium 155 preserves encoded orientation for 32 valid WebP EXIF-orientation fixtures; not a general EXIF policy.
use super::*;
use base64::Engine;

// Chromium Canvas-generated 3x2 six-color lossless WebP with sRGB ICC and little-endian orientation 1.
// See SURFACE_WEBP_ISOLATION.md for browser baselines and independent EXIF-read records.
const BASE: &str = "UklGRjACAABXRUJQVlA4WAoAAAAoAAAAAgAAAQAASUNDUMgBAAAAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADZWUDhMIAAAAC8CQAAALyAQSNofeo353//5Dx5UaEBAQqbkgk0i+h+xRVhJRhoAAABJSSoACAAAAAEAEgEDAAEAAAABAAAAAAAAAA==";

#[test]
fn surface_webp_exif_eight_directions_match_chromium_encoded_order() {
    let original = base64::engine::general_purpose::STANDARD
        .decode(BASE)
        .unwrap();
    let end = original.len() - 34;
    assert_eq!(&original[end..end + 4], b"EXIF");
    for prefix in [false, true] {
        for little in [true, false] {
            for direction in 1..=8 {
                let mut tiff = if little {
                    *b"II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\0\0\0\0\0\0\0\0"
                } else {
                    *b"MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\0\0\0\0\0\0\0"
                };
                tiff[if little { 18 } else { 19 }] = direction;
                let mut metadata = if prefix {
                    b"Exif\0\0".to_vec()
                } else {
                    Vec::new()
                };
                metadata.extend_from_slice(&tiff);
                let mut bytes = original[..end].to_vec();
                bytes.extend_from_slice(b"EXIF");
                bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
                bytes.extend_from_slice(&metadata);
                let size = (bytes.len() - 8) as u32;
                bytes[4..8].copy_from_slice(&size.to_le_bytes());
                if !prefix && little && direction == 1 {
                    assert_eq!(bytes, original);
                }
                let image = Image::from_surface(&bytes, "image/webp", None, || false).unwrap();
                assert_eq!((image.width, image.height), (3, 2));
                assert_eq!(
                    &*image.pixels,
                    &[
                        0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 255, 255, 255, 255, 0,
                        255, 255, 255, 255, 0, 255,
                    ],
                    "prefix={prefix} little={little} direction={direction}"
                );
            }
        }
    }
}
