use super::*;
use crate::editor::Tool;

fn source() -> Image {
    let mut pixels = vec![0; 32 * 32 * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[20, 40, 60, 128]);
    }
    Image {
        width: 32,
        height: 32,
        pixels: pixels.into(),
    }
}

pub(in crate::desktop) fn decode_png(png: Vec<u8>) -> Image {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = wic.CreateStream().unwrap();
        stream.InitializeFromMemory(&png).unwrap();
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
            .unwrap();
        let frame = decoder.GetFrame(0).unwrap();
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height).unwrap();
        let converter = wic.CreateFormatConverter().unwrap();
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .unwrap();
        let mut pixels = vec![0; width as usize * height as usize * 4];
        converter
            .CopyPixels(std::ptr::null(), width * 4, &mut pixels)
            .unwrap();
        Image {
            width,
            height,
            pixels: pixels.into(),
        }
    }
}

#[test]
fn mosaic_single_click_is_opaque_and_selectable_even_with_zero_shared_opacity() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 32,
        height: 32,
        pixels: vec![0; 32 * 32 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        editor
            .style()
            .with_width(12.0)
            .unwrap()
            .with_opacity(0.0)
            .unwrap(),
    );
    editor
        .begin(Tool::Mosaic, Point { x: 8.0, y: 8.0 })
        .unwrap();
    assert!(editor.commit());
    let output = decode_png(encode(&image, &editor).unwrap());
    assert_eq!(output.pixels[(8 * 32 + 8) * 4 + 3], 255);
    assert_eq!(output.pixels[(20 * 32 + 20) * 4 + 3], 0);
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        assert_eq!(
            super::super::selection_hit::pick(&factory, &editor, Point { x: 8.0, y: 8.0 }, 0.0)
                .unwrap(),
            Some(0)
        );
    }
}

#[test]
fn mosaic_png_masks_source_without_sampling_and_preserves_history() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let other = Image {
        width: 64,
        height: 64,
        pixels: vec![255; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        editor
            .style()
            .with_width(20.0)
            .unwrap()
            .with_opacity(0.1)
            .unwrap()
            .with_dash(crate::stroke_style::DashPattern::Dash),
    );
    editor
        .begin(Tool::Mosaic, Point { x: 10.0, y: 20.0 })
        .unwrap();
    editor.extend(Point { x: 32.0, y: 20.0 }).unwrap();
    editor.extend(Point { x: 50.0, y: 20.0 }).unwrap();
    assert!(editor.commit());
    assert_eq!(editor.elements()[0].points.len(), 3);
    let output = decode_png(encode(&image, &editor).unwrap());
    let other_output = decode_png(encode(&other, &editor).unwrap());
    let pattern = crate::mosaic::generate(64, 64, 12.0, Point::default()).unwrap();
    for y in 14..26 {
        for x in 10..50 {
            let i = (y * 64 + x) * 4;
            assert_eq!(&output.pixels[i..i + 4], &pattern[i..i + 4]);
            assert_eq!(&output.pixels[i..i + 4], &other_output.pixels[i..i + 4]);
        }
    }
    assert_eq!(output.pixels[(50 * 64 + 50) * 4 + 3], 0);
    editor.undo();
    assert!(decode_png(encode(&image, &editor).unwrap())
        .pixels
        .iter()
        .all(|b| *b == 0));
    editor.redo();
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        output.pixels
    );
}

#[test]
fn serial_exports_circle_and_number_and_undo_restores_transparent_image() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor
        .insert_serial(Point { x: 32.0, y: 32.0 }, 14.0, "微软雅黑")
        .unwrap();
    let output = decode_png(encode(&image, &editor).unwrap());
    assert_eq!(output.pixels[3], 0);
    assert_eq!(output.pixels[(32 * 64 + 32) * 4 + 3], 255);
    assert!(output
        .pixels
        .chunks_exact(4)
        .any(|pixel| pixel[2] > 100 && pixel[3] > 0));
    editor.undo();
    assert!(decode_png(encode(&image, &editor).unwrap())
        .pixels
        .iter()
        .all(|byte| *byte == 0));
    editor.redo();
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        output.pixels
    );
}

#[test]
fn native_text_renders_to_png_and_undo_removes_ink() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 128,
        height: 64,
        pixels: vec![0; 128 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    let text =
        crate::text_annotation::TextAnnotation::new("你好 Otter", "Segoe UI", 16.0, false).unwrap();
    editor
        .insert_text(Point { x: 4.0, y: 4.0 }, Point { x: 120.0, y: 56.0 }, text)
        .unwrap();
    let rendered = decode_png(encode(&image, &editor).unwrap());
    assert!(rendered.pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    editor.undo();
    let cleared = decode_png(encode(&image, &editor).unwrap());
    assert!(cleared.pixels.iter().all(|byte| *byte == 0));
    editor.redo();
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        rendered.pixels
    );
}

#[test]
fn restyled_annotation_exports_new_pixels_and_undo_restores_original() {
    use crate::stroke_style::Color;
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 32,
        height: 32,
        pixels: vec![0; 32 * 32 * 4].into(),
    };
    let mut editor = Editor::default();
    editor
        .begin(Tool::Brush, Point { x: 16.0, y: 16.0 })
        .unwrap();
    editor.commit();
    let before = decode_png(encode(&image, &editor).unwrap());
    let style = editor
        .style()
        .with_color(Color(255, 0, 0))
        .with_width(12.0)
        .unwrap();
    assert!(editor.restyle(0, style));
    let after = decode_png(encode(&image, &editor).unwrap());
    let offset = (16 * 32 + 20) * 4;
    assert_eq!(before.pixels[offset + 3], 0);
    assert_eq!(&after.pixels[offset..offset + 4], &[0, 0, 255, 255]);
    editor.undo();
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        before.pixels
    );
    editor.redo();
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        after.pixels
    );
}

#[test]
fn png_roundtrip_preserves_size_and_premultiplied_alpha() {
    let _apartment = Apartment::new().unwrap();
    let source = source();
    let decoded = decode_png(encode(&source, &Editor::default()).unwrap());
    assert_eq!((decoded.width, decoded.height), (32, 32));
    for (actual, expected) in decoded.pixels.iter().zip(source.pixels.iter()) {
        assert!(actual.abs_diff(*expected) <= 1, "{actual} != {expected}");
    }
}

#[test]
fn each_annotation_exports_its_own_color_and_width() {
    use crate::stroke_style::{Color, StrokeStyle};
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 32,
        pixels: vec![0; 64 * 32 * 4].into(),
    };
    let mut editor = Editor::default();
    for (x, width, color) in [
        (16.0, 12.0, Color(255, 0, 0)),
        (48.0, 2.0, Color(0, 0, 255)),
    ] {
        editor.set_style(StrokeStyle::new(width, color).unwrap());
        editor.begin(Tool::Brush, Point { x, y: 16.0 }).unwrap();
        editor.commit();
    }
    let rendered = decode_png(encode(&image, &editor).unwrap());
    let pixel = |x: usize, y: usize| &rendered.pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert_eq!(pixel(16, 16), &[0, 0, 255, 255]);
    // 2 像素圆点位于像素交界处，覆盖率由抗锯齿决定；预乘蓝色应匹配 alpha。
    assert!(pixel(48, 16)[3] > 0);
    assert!(pixel(48, 16)[0].abs_diff(pixel(48, 16)[3]) <= 1);
    assert_eq!(pixel(48, 16)[2], 0);
    assert_eq!(pixel(20, 16)[3], 255);
    assert_eq!(pixel(52, 16)[3], 0);
}

#[test]
fn arrow_head_is_filled_and_survives_undo_redo_export() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 32,
        pixels: vec![0; 64 * 32 * 4].into(),
    };
    let mut editor = Editor::default();
    editor
        .begin(Tool::Arrow, Point { x: 4.0, y: 16.0 })
        .unwrap();
    editor.extend(Point { x: 56.0, y: 16.0 }).unwrap();
    assert!(editor.commit());
    let rendered = decode_png(encode(&image, &editor).unwrap());
    let alpha = |x: usize, y: usize| rendered.pixels[(y * 64 + x) * 4 + 3];
    assert_eq!(alpha(36, 10), 255, "filled head extends outside shaft");
    assert_eq!(alpha(16, 10), 0, "shaft does not fill the entire bounds");
    assert!(editor.undo());
    let empty = decode_png(encode(&image, &editor).unwrap());
    assert!(empty.pixels.iter().all(|value| *value == 0));
    assert!(editor.redo());
    assert_eq!(
        decode_png(encode(&image, &editor).unwrap()).pixels,
        rendered.pixels
    );
}

#[test]
fn export_contains_committed_dab_but_excludes_draft_and_redo() {
    let _apartment = Apartment::new().unwrap();
    let mut editor = Editor::default();
    editor.begin(Tool::Brush, Point { x: 8.0, y: 8.0 }).unwrap();
    editor.commit();
    editor
        .begin(Tool::Brush, Point { x: 24.0, y: 24.0 })
        .unwrap();
    editor.commit();
    editor.undo();
    editor
        .begin(Tool::Brush, Point { x: 16.0, y: 16.0 })
        .unwrap();
    let snapshot = editor.export_snapshot();
    assert!(snapshot.draft().is_none());
    let decoded = decode_png(encode(&source(), &editor).unwrap());
    let pixel = |x: usize, y: usize| &decoded.pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
    assert_eq!(pixel(8, 8)[3], 255);
    assert_eq!(pixel(8, 8), &[68, 68, 239, 255]);
    assert_eq!(pixel(16, 16)[3], 128);
    assert_eq!(pixel(24, 24)[3], 128);
}

#[test]
fn every_basic_shape_produces_pixels_in_native_png() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Line, Tool::Rectangle, Tool::Ellipse] {
        let mut editor = Editor::default();
        editor.begin(tool, Point { x: 4.0, y: 4.0 }).unwrap();
        editor.extend(Point { x: 20.0, y: 20.0 }).unwrap();
        assert!(editor.commit());
        let decoded = decode_png(encode(&source(), &editor).unwrap());
        assert!(
            decoded
                .pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[3] == 255)
                .count()
                > 20
        );
        assert_eq!(
            &decoded.pixels[(31 * 32 + 31) * 4..][..4],
            &[20, 40, 60, 128]
        );
    }
}

fn output_path(name: &str) -> std::path::PathBuf {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("otter-export-tests");
    std::fs::create_dir_all(&root).unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    root.join(format!("{name}-{}-{nonce}.png", std::process::id()))
}

#[test]
fn save_is_readable_and_refuses_to_overwrite_existing_file() {
    let path = output_path("中文导出");
    save_new(&path, &source(), &Editor::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(save_new(&path, &source(), &Editor::default()).is_err());
    assert_eq!(before, std::fs::read(&path).unwrap());
    let _apartment = Apartment::new().unwrap();
    let loaded = Image::open(&path).unwrap();
    assert_eq!((loaded.width, loaded.height), (32, 32));
}

#[test]
fn invalid_source_is_rejected_before_output_creation() {
    let path = output_path("invalid");
    let bad = Image {
        width: 32,
        height: 32,
        pixels: vec![0; 4].into(),
    };
    assert!(save_new(&path, &bad, &Editor::default()).is_err());
    assert!(!path.exists());
}

#[test]
fn stream_copy_reports_truncation_and_write_failure() {
    let _apartment = Apartment::new().unwrap();
    let (stream, size) = encode_stream(&source(), &Editor::default()).unwrap();
    assert!(copy_stream(&stream, size + 1, &mut Vec::new()).is_err());
    struct FailedWriter;
    impl std::io::Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    unsafe {
        stream.Seek(0, STREAM_SEEK_SET, None).unwrap();
    }
    assert!(copy_stream(&stream, size, &mut FailedWriter)
        .unwrap_err()
        .contains("injected write failure"));
}
