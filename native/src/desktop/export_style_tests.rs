//! 原生像素证明：样式以 unit DIP 计，边框在独立标注下，圆角裁整个栈。
use super::*;
use serde_json::json;
fn image() -> Image {
    Image {
        width: 40,
        height: 20,
        pixels: [255, 255, 255, 255].repeat(800).into(),
    }
}
fn styled() -> Snapshot {
    Snapshot {
        style: Style::read(&json!({"borderWidth":2,"borderColor":"red","cornerRadius":4})).unwrap(),
        logical: (20., 10.),
    }
}
fn pixels(s: Snapshot, image: &Image, editor: &Editor, opacity: f32) -> Image {
    let _com = Apartment::new().unwrap();
    let (png, _) = s.png(image, editor, opacity).unwrap();
    super::super::decode_png(png.to_vec())
}
fn at(image: &Image, x: usize, y: usize) -> &[u8] {
    &image.pixels[(y * image.width as usize + x) * 4..(y * image.width as usize + x + 1) * 4]
}
#[test]
fn radius_masks_full_stack_and_border_uses_logical_density_without_inheriting_base_opacity() {
    let base = image();
    let out = pixels(styled(), &base, &Editor::default(), 0.5);
    assert_eq!((out.width, out.height), (40, 20));
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert_eq!(at(&out, 20, 1), &[0, 0, 255, 255]);
    assert_eq!(at(&out, 20, 3), &[0, 0, 255, 255]);
    assert!(at(&out, 20, 6)[3].abs_diff(128) <= 1);
    assert!(at(&out, 20, 6)[0].abs_diff(128) <= 1);
    let mut overlay = vec![0; 3200];
    overlay[(20 + 40) * 4..(21 + 40) * 4].copy_from_slice(&[255, 0, 0, 255]);
    overlay[..4].copy_from_slice(&[255, 0, 0, 255]);
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (40, 20),
        base: base.pixels.clone(),
        annotations: Some(overlay.into()),
        annotation_size: None,
    })
    .unwrap();
    let out = pixels(styled(), &base, &editor, 0.5);
    assert_eq!(at(&out, 20, 1), &[255, 0, 0, 255]);
    assert_eq!(at(&out, 0, 0)[3], 0);
    assert!(editor.layers().unwrap().base.same_storage(&base.pixels));
}
#[test]
fn fractional_crop_and_nonuniform_density_keep_border_in_unit_coordinates() {
    let base = image();
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (40, 20),
        base: base.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 5.,
            y: 2.,
            width: 20.,
            height: 16.,
        })
        .unwrap();
    let mut s = styled();
    s.logical = (10., 16.);
    s.style = Style::read(&json!({"borderWidth":2,"borderColor":"red"})).unwrap();
    let out = pixels(s, &base, &editor, 1.);
    assert_eq!((out.width, out.height), (20, 16));
    assert_eq!(at(&out, 2, 8), &[0, 0, 255, 255]);
    assert_eq!(at(&out, 10, 1), &[0, 0, 255, 255]);
    assert_eq!(at(&out, 10, 3), &[255, 255, 255, 255]);
}
#[test]
fn corner_layer_budget_is_released_on_failure_and_success_and_bad_unit_size_is_rejected() {
    let _com = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let base = image();
    let editor = Editor::default();
    let budget = crate::byte_budget::Budget::new(6400);
    assert!(styled()
        .compose(&base, &editor, &wic, 1., |n| budget.reserve(n))
        .is_err());
    assert_eq!(budget.used(), 0);
    let budget = crate::byte_budget::Budget::new(9600);
    let out = styled()
        .compose(&base, &editor, &wic, 1., |n| budget.reserve(n))
        .unwrap();
    assert_eq!(budget.used(), 3200);
    drop(out);
    assert_eq!(budget.used(), 0);
    let mut bad = styled();
    bad.logical = (f64::NAN, 10.);
    assert!(bad
        .compose(&base, &editor, &wic, 1., crate::pixels::reserve)
        .is_err());
}
#[test]
fn editable_clipboard_preserves_style_without_double_baking_pixels() {
    let base = image();
    let style = styled();
    let content = crate::desktop::clipboard_editable::Content {
        image: base.clone(),
        editor: Editor::default(),
        opacity: 0.5,
        frame_style: style.style,
        capture_source: None,
        origin: None,
    };
    let prepared = content.prepare(|| false).unwrap();
    assert!(prepared.image.pixels.same_storage(&base.pixels));
    let state = crate::desktop::State::from_prepared(prepared).unwrap();
    let mut c = state.canvas.borrow_mut();
    assert_eq!(c.frame_style, style.style);
    c.client_size = Some(Point { x: 20., y: 10. });
    let restored = Snapshot::capture(&c);
    assert_eq!(restored, style);
    assert_eq!(
        &*pixels(restored, &base, &c.editor, 0.5).pixels,
        &*pixels(style, &base, &Editor::default(), 0.5).pixels
    );
}
