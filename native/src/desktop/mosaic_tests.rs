//! 真实 WIC 输出验证分块接缝和逆旋转裁剪，不读取用户图片。
use super::*;
use crate::editor::Tool;

fn image() -> Image {
    Image {
        width: 520,
        height: 64,
        pixels: vec![0; 520 * 64 * 4].into(),
    }
}
fn long_path() -> Editor {
    let mut editor = Editor::default();
    editor.set_style(editor.style().with_width(20.0).unwrap());
    editor
        .begin(
            Tool::Mosaic,
            Point {
                x: -900_000.0,
                y: 32.0,
            },
        )
        .unwrap();
    editor
        .extend(Point {
            x: 900_520.0,
            y: 32.0,
        })
        .unwrap();
    assert!(editor.commit());
    editor
}
#[test]
fn mosaic_cell_is_per_stroke_and_restyle_has_exact_undo() {
    let _apartment = Apartment::new().unwrap();
    let image = image();
    let mut editor = long_path();
    let initial = tests::decode_png(encode(&image, &editor).unwrap());
    let revision = editor.revision();
    assert!(editor.style().with_mosaic_cell(1).is_none());
    assert!(editor.style().with_mosaic_cell(65).is_none());
    editor.set_style(editor.style().with_mosaic_cell(24).unwrap());
    assert_eq!(editor.revision(), revision);
    assert_eq!(editor.elements()[0].mosaic_cell, 12.0);
    assert_eq!(
        tests::decode_png(encode(&image, &editor).unwrap()).pixels,
        initial.pixels
    );
    assert!(editor.restyle(0, editor.style()));
    assert_eq!(editor.elements()[0].mosaic_cell, 24.0);
    let changed = tests::decode_png(encode(&image, &editor).unwrap());
    assert_ne!(changed.pixels, initial.pixels);
    assert!(editor.undo());
    assert_eq!(editor.elements()[0].mosaic_cell, 12.0);
    assert_eq!(
        tests::decode_png(encode(&image, &editor).unwrap()).pixels,
        initial.pixels
    );
    assert!(editor.redo());
    assert_eq!(
        tests::decode_png(encode(&image, &editor).unwrap()).pixels,
        changed.pixels
    );
}

#[test]
fn huge_path_is_clipped_before_allocation_and_tiles_have_no_seams() {
    let _apartment = Apartment::new().unwrap();
    let image = image();
    let output = tests::decode_png(encode(&image, &long_path()).unwrap());
    let pattern = crate::mosaic::generate(520, 64, 12.0, Point::default()).unwrap();
    for y in 25..39 {
        for x in 0..520 {
            let i = (y * 520 + x) * 4;
            assert_eq!(
                &output.pixels[i..i + 4],
                &pattern[i..i + 4],
                "pixel {x},{y}"
            );
        }
    }
    assert_eq!(output.pixels[3], 0);
}
#[test]
fn inverse_rotation_preserves_visible_part_of_huge_path() {
    let _apartment = Apartment::new().unwrap();
    let image = image();
    let mut editor = long_path();
    editor.rotate(0, 90.0).unwrap();
    let output = tests::decode_png(encode(&image, &editor).unwrap());
    for y in 0..64 {
        for x in 254..266 {
            assert_eq!(output.pixels[(y * 520 + x) * 4 + 3], 255, "pixel {x},{y}");
        }
    }
    assert_eq!(output.pixels[(32 * 520 + 30) * 4 + 3], 0);
}
