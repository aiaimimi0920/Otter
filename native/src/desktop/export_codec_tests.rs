use super::*;
fn source() -> Source {
    let mut bytes = Vec::new();
    for _ in 0..32 {
        for x in 0..48 {
            bytes.extend_from_slice(if x < 16 {
                &[0, 0, 0, 0]
            } else if x < 32 {
                &[0, 0, 64, 128]
            } else {
                &[7, 80, 143, 255]
            });
        }
    }
    Source {
        image: Image {
            width: 48,
            height: 32,
            pixels: bytes.into(),
        },
        editor: Editor::default(),
        style: Default::default(),
        opacity: 1.,
    }
}
#[test]
fn webp_and_jpeg_decode_the_same_display_with_explicit_alpha_rules() {
    let _apt = Apartment::new().unwrap();
    let s = source();
    let stop = AtomicBool::new(false);
    let png = s.encode(Format::Png, &stop).unwrap();
    let png = super::super::decode_png(png.to_vec());
    assert_eq!(&png.pixels[24 * 4..25 * 4], &[0, 0, 64, 128]);
    let bytes = s.encode(Format::WebpLossless, &stop).unwrap();
    assert_eq!(&bytes[8..16], b"WEBPVP8L");
    let mut d = image_webp::WebPDecoder::new(io::Cursor::new(&*bytes)).unwrap();
    assert_eq!(d.dimensions(), (48, 32));
    let mut rgba = vec![0; 48 * 32 * 4];
    d.read_image(&mut rgba).unwrap();
    assert_eq!(&rgba[..4], &[0, 0, 0, 0]);
    assert_eq!(&rgba[24 * 4..25 * 4], &[128, 0, 0, 128]);
    let bytes = s.encode(Format::Jpeg(100), &stop).unwrap();
    assert_eq!(&bytes[..3], &[255, 216, 255]);
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = wic.CreateStream().unwrap();
        stream.InitializeFromMemory(&bytes).unwrap();
        let frame = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .unwrap()
            .GetFrame(0)
            .unwrap();
        let conv = wic.CreateFormatConverter().unwrap();
        conv.Initialize(
            &frame,
            &GUID_WICPixelFormat24bppBGR,
            WICBitmapDitherTypeNone,
            None,
            0.,
            WICBitmapPaletteTypeCustom,
        )
        .unwrap();
        let mut bgr = vec![0u8; 48 * 32 * 3];
        conv.CopyPixels(std::ptr::null(), 48 * 3, &mut bgr).unwrap();
        assert!(bgr[8 * 3..9 * 3].iter().all(|v| *v >= 253));
        for (a, b) in bgr[24 * 3..25 * 3].iter().zip([127, 127, 191]) {
            assert!(a.abs_diff(b) <= 3);
        }
    }
    assert_ne!(&*bytes, &*s.encode(Format::Jpeg(20), &stop).unwrap());
    assert!(s.encode(Format::Jpeg(0), &stop).is_err());
    assert!(s.encode(Format::Png, &AtomicBool::new(true)).is_err());
    assert!(Format::Jpeg(90).accepts(Path::new("image.JPEG")));
    assert!(!Format::WebpLossless.accepts(Path::new("image.jpg")));
}
#[test]
fn lossy_vp8_has_separate_quality_and_byte_exact_alpha() {
    let s = source();
    let stop = AtomicBool::new(false);
    let hi = s.encode(Format::WebpLossy(100), &stop).unwrap();
    let lo = s.encode(Format::WebpLossy(1), &stop).unwrap();
    assert_ne!(hi, lo);
    for bytes in [lo, hi] {
        let mut pos = 12usize;
        let mut chunks = Vec::new();
        while pos < bytes.len() {
            let n = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
            chunks.push(&bytes[pos..pos + 4]);
            pos += 8 + n + (n & 1);
        }
        assert_eq!(chunks, [b"VP8X", b"ALPH", b"VP8 "]);
        assert_eq!(pos, bytes.len());
        let mut d = image_webp::WebPDecoder::new(io::Cursor::new(&*bytes)).unwrap();
        let mut out = vec![0; 48 * 32 * 4];
        d.read_image(&mut out).unwrap();
        assert!(out
            .chunks_exact(4)
            .zip(s.image.pixels.chunks_exact(4))
            .all(|(actual, source)| actual[3] == source[3]));
    }
    assert!(s.encode(Format::WebpLossy(0), &stop).is_err());
}

#[test]
fn rotated_cropped_lens_at_erasure_edge_matches_flattened_clipboard_and_all_codecs() {
    use crate::{
        editor::Tool,
        erase_layers::{Layers, Mode},
    };
    let _apt = Apartment::new().unwrap();
    let stop = AtomicBool::new(false);
    let p = |x, y| Point { x, y };
    let image = Image {
        width: 64,
        height: 48,
        pixels: (0..64 * 48)
            .flat_map(|i| match i % 64 {
                32.. => [0_u8; 4],
                x if x % 2 == 0 => [255, 255, 255, 255],
                _ => [0, 0, 0, 255],
            })
            .collect::<Vec<_>>()
            .into(),
    };
    let before = Layers {
        size: (64, 48),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    };
    let after = before
        .erase(Mode::Content, &[p(16.5, 22.5)], 16., || false)
        .unwrap();
    let pixel = |image: &Image, x: usize, y: usize| {
        let i = (image.width as usize * y + x) * 4;
        <[u8; 4]>::try_from(&image.pixels[i..i + 4]).unwrap()
    };
    let erased = Image {
        width: 64,
        height: 48,
        pixels: after.base.clone(),
    };
    assert!(
        (1..255).contains(&pixel(&erased, 24, 22)[3]),
        "fixture must sample actual AA coverage"
    );
    let mut editor = Editor::with_layers(before.clone()).unwrap();
    editor
        .replace_layers(editor.revision(), &before, after)
        .unwrap();
    editor.begin(Tool::Magnifier, p(41., 14.)).unwrap();
    editor.extend(p(58., 31.)).unwrap();
    assert!(editor.commit());
    editor.rotate(0, 90.).unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 4.,
            y: 4.,
            width: 56.,
            height: 40.,
        })
        .unwrap();
    for x in [23usize, 24, 25] {
        let center = p(49.5, 22.5);
        let source = crate::rotation::rotate(p(x as f32 + 0.5, 22.5), center, -90.);
        editor.move_lens_handle(0, 0, source).unwrap();
        let source = Source {
            image: image.clone(),
            editor: editor.export_snapshot(),
            style: Default::default(),
            opacity: 1.,
        };
        let flat = crate::desktop::display_composition::compose(&image, &editor, &stop).unwrap();
        assert_eq!((flat.width, flat.height), (56, 40));
        let expected = pixel(&erased, x, 22);
        let check = |frame: &Image| {
            let actual = pixel(frame, 45, 18);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 2),
                "edge {x}: actual={actual:?}, expected={expected:?}"
            );
        };
        check(&flat);
        let clipboard = source.style.clipboard(&image, &editor, 1.).unwrap();
        check(&super::super::decode_png(clipboard.png.to_vec()));
        check(
            &crate::desktop::clipboard_dib::decode_with_cancel(&clipboard.dibv5, || false).unwrap(),
        );
        let projected = crate::desktop::export::projection::frame(
            &erased,
            &editor,
            1.,
            crate::projection_delivery::sender::Kind::Sticker,
            &stop,
        )
        .unwrap();
        check(&super::super::decode_png(projected.png.to_vec()));
        let safe = Source {
            image: flat,
            editor: Editor::default(),
            style: Default::default(),
            opacity: 1.,
        };
        // Compare encoded bytes against the already checked PBGRA composite:
        // lossy quantization is deterministic and cannot serve as a privacy tolerance.
        for format in [
            Format::Png,
            Format::Jpeg(91),
            Format::WebpLossless,
            Format::WebpLossy(83),
        ] {
            assert_eq!(
                &*source.encode(format, &stop).unwrap(),
                &*safe.encode(format, &stop).unwrap(),
                "{format:?} at {x}"
            );
        }
    }
}
