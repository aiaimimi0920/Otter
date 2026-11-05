//! 所有效果创建顺序都必须保持遮挡层级，命中对象与可见最上层一致。
use super::*;
use crate::{editor::Tool, stroke_style::Color};

#[test]
fn blur_cannot_cover_mosaic_and_ordinary_annotations_stay_above_both() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 32,
        height: 32,
        pixels: vec![0; 32 * 32 * 4].into(),
    };
    let blur = crate::blur_image::BlurImage::new(32, 32, vec![255; 32 * 32 * 4].into(), 8).unwrap();
    let point = Point { x: 16.0, y: 16.0 };
    let pattern = crate::mosaic::generate(32, 32, 12.0, Point::default()).unwrap();
    let pixel = (16 * 32 + 16) * 4;
    let factory: ID2D1Factory =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() };
    for order in [
        [Tool::Brush, Tool::Mosaic, Tool::Blur],
        [Tool::Brush, Tool::Blur, Tool::Mosaic],
        [Tool::Mosaic, Tool::Brush, Tool::Blur],
        [Tool::Mosaic, Tool::Blur, Tool::Brush],
        [Tool::Blur, Tool::Brush, Tool::Mosaic],
        [Tool::Blur, Tool::Mosaic, Tool::Brush],
    ] {
        let mut editor = Editor::default();
        editor.set_style(
            editor
                .style()
                .with_width(16.0)
                .unwrap()
                .with_color(Color(255, 0, 0)),
        );
        for tool in order {
            if tool == Tool::Blur {
                editor.begin_blur(point, blur.clone()).unwrap();
            } else {
                editor.begin(tool, point).unwrap();
            }
            assert!(editor.commit());
        }
        let brush = editor
            .elements()
            .iter()
            .position(|s| s.tool == Tool::Brush)
            .unwrap();
        let output = tests::decode_png(encode(&source, &editor).unwrap());
        assert_eq!(
            &output.pixels[pixel..pixel + 4],
            &[0, 0, 255, 255],
            "{order:?}"
        );
        assert_eq!(
            unsafe { super::super::selection_hit::pick(&factory, &editor, point, 0.0).unwrap() },
            Some(brush)
        );
        editor.remove(brush);
        let mosaic = editor
            .elements()
            .iter()
            .position(|s| s.tool == Tool::Mosaic)
            .unwrap();
        let output = tests::decode_png(encode(&source, &editor).unwrap());
        assert_eq!(
            &output.pixels[pixel..pixel + 4],
            &pattern[pixel..pixel + 4],
            "{order:?}"
        );
        assert_eq!(
            unsafe { super::super::selection_hit::pick(&factory, &editor, point, 0.0).unwrap() },
            Some(mosaic)
        );
        editor.remove(mosaic);
        let visible_blur = tests::decode_png(encode(&source, &editor).unwrap());
        assert_eq!(&visible_blur.pixels[pixel..pixel + 4], &[255; 4]);
        editor.undo();
        assert_eq!(
            tests::decode_png(encode(&source, &editor).unwrap()).pixels,
            output.pixels
        );
    }
}
