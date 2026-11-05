//! 主干底图透明度合同：底图单独衰减，栅格和矢量标注保持自身透明度。
use super::*;
use crate::{editor::Tool, erase_layers::Layers, stroke_style::Color};

#[test]
fn output_opacity_changes_only_base_without_mutating_source() {
    let _apartment = Apartment::new().unwrap();
    let mut raster = vec![0; 16 * 16 * 4];
    raster[..4].copy_from_slice(&[0, 255, 0, 255]);
    let layers = Layers {
        size: (16, 16),
        annotation_size: None,
        base: [0, 0, 255, 255].repeat(16 * 16).into(),
        annotations: Some(raster.into()),
    };
    let image = Image {
        width: 16,
        height: 16,
        pixels: layers.base.clone(),
    };
    let mut editor = Editor::with_layers(layers.clone()).unwrap();
    editor.set_style(
        editor
            .style()
            .with_width(4.0)
            .unwrap()
            .with_color(Color(0, 0, 255)),
    );
    editor.begin(Tool::Brush, Point { x: 8.0, y: 8.0 }).unwrap();
    assert!(editor.commit());
    let stamp = editor.document_stamp();
    for opacity in [0.0, 0.5, 1.0] {
        let payload = clipboard_payload_with_opacity(&image, &editor, opacity).unwrap();
        let output = tests::decode_png(payload.png.to_vec());
        assert_eq!(&output.pixels[..4], &[0, 255, 0, 255]);
        let vector = (8 * 16 + 8) * 4;
        assert_eq!(&output.pixels[vector..vector + 4], &[255, 0, 0, 255]);
        let base = (15 * 16 + 15) * 4;
        let alpha = (opacity * 255.0).round() as u8;
        assert!(output.pixels[base + 3].abs_diff(alpha) <= 1);
        assert!(output.pixels[base + 2].abs_diff(alpha) <= 1);
        assert!(stamp.matches(&editor));
        assert!(editor.layers().unwrap().base.same_storage(&layers.base));
    }
}

#[test]
fn invalid_base_opacity_is_rejected_before_pixel_allocation() {
    let _apartment = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    };
    for opacity in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(
            compose_opacity(&image, &Editor::default(), &wic, opacity, |_| panic!(
                "invalid opacity must not reserve pixels"
            ))
            .is_err()
        );
    }
}
