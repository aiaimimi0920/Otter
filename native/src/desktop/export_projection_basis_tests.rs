//! 无矢量basis独立像素/几何oracle；不通过新basis renderer生成预期值。
use super::*;
use crate::crop_geometry::Rect;
use serde_json::json;
#[test]
fn basis_excludes_vector_and_normal_opacity_but_keeps_raster_annotations_crop_and_active_editor() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 8,
        pixels: [255, 0, 0, 255].repeat(64).into(),
    };
    let mut raster = vec![0; 256];
    raster[(3 * 8 + 3) * 4..(3 * 8 + 3) * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some(raster.into()),
        annotation_size: None,
    })
    .unwrap();
    editor.set_style(StrokeStyle::new(2.0, Color(255, 0, 0)).unwrap());
    editor.begin(Tool::Brush, Point { x: 5.0, y: 5.0 }).unwrap();
    assert!(editor.commit());
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let before = editor.document_stamp();
    let source = br#"{"stickers":[{"id":"basis"}]}"#.to_vec().into();
    let (frame, layout) = basis_frame(
        &source,
        "basis",
        &image,
        &editor,
        Presentation {
            opacity: 0.25,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        Input::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((frame.width, frame.height), (4, 4));
    let decoded = crate::desktop::export::decode_png(frame.png.to_vec());
    for y in 0..4 {
        for x in 0..4 {
            let expected = if (x, y) == (1, 1) {
                // 原栅格层完整8x8映到4x4可见框；双线性在(2.5,2.5)取样，绿色权重1/4。
                // source-over: green=round(255/4)=64，blue=255-64=191；不是源框直裁剪。
                [191, 64, 0, 255]
            } else {
                [255, 0, 0, 255]
            };
            let i = (y * 4 + x) * 4;
            assert_eq!(&decoded.pixels[i..i + 4], &expected);
        }
    }
    assert_eq!(
        layout.size().unwrap(),
        crate::projection_edit::view::Size::new(4.0, 4.0).unwrap()
    );
    assert_eq!(
        layout.mapping().unwrap().components().1,
        Point { x: -2.0, y: -2.0 }
    );
    assert!(before.matches(&editor));
    assert_eq!(editor.elements().len(), 1);
}
#[test]
fn basis_geometry_uses_actual_nonuniform_crop_density_and_pixel_padding_for_initial_vectors() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 16,
        height: 8,
        pixels: [255, 0, 0, 255].repeat(128).into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: [255, 0, 0, 255].repeat(64).into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let source=serde_json::to_vec(&json!({"stickers":[{"id":"basis","imageEditState":{"beautify":{"enabled":true,"padding":2,"radius":0,"shadow":false}}}]})).unwrap().into();
    let (frame, layout) = basis_frame(
        &source,
        "basis",
        &image,
        &editor,
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        Input::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    // 16/8源密度=2，4x4裁剪输出8x8，再各侧2px；view为6x6，原点-1,-1。
    assert_eq!((frame.width, frame.height), (12, 12));
    assert_eq!(
        layout.size().unwrap(),
        crate::projection_edit::view::Size::new(6.0, 6.0).unwrap()
    );
    assert_eq!(
        layout.mapping().unwrap().components().1,
        Point { x: -1.0, y: -1.0 }
    );
    let decoded = crate::desktop::export::decode_png(frame.png.to_vec());
    assert_eq!((decoded.width, decoded.height), (12, 12));
}
