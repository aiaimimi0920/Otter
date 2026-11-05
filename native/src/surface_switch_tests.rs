//! Switch 的声明值、布局与真实 D2D 像素；不把绘制支持当作交互准入。
use super::*;

#[test]
fn surface_switch_boolean_defaults_and_native_measurement() {
    for (value, expected) in [
        ("true", true),
        ("false", false),
        ("1", false),
        ("\"true\"", false),
        ("null", false),
    ] {
        let json = format!(
            r#"{{"id":"switch","type":"switch","props":{{"value":{value}}},"layout":{{"width":200,"height":200}},"style":{{"padding":90,"background":"red"}}}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        assert_eq!(crate::surface_switch::checked(scene.root()), expected);
        let mut measure = NativeMeasure::new(&scene, base(), context(), [64., 64.]).unwrap();
        assert_eq!(
            measure
                .measure(scene.root(), [64., 64.], context())
                .unwrap()
                .size,
            [34., 19.]
        );
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
                .unwrap();
        assert!(presentation.text().items().is_empty());
        assert!(presentation.focusable("switch"));
    }
}

#[test]
fn surface_switch_pixels_distinguish_checked_thumb_without_new_text_resources() {
    let render = |checked| {
        super::paint_pixels::render(
            format!(r#"{{"id":"switch","type":"switch","props":{{"value":{checked}}}}}"#)
                .as_bytes(),
        )
    };
    let off = render(false);
    let on = render(true);
    let pixel =
        |pixels: &[u8], x: usize, y: usize| pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4].to_vec();
    // 根宽 64，固定 34 DIP 轨道在 label 中水平居中，根自然高 19。
    assert_eq!(pixel(&off, 24, 9), [159, 154, 146, 255]);
    assert_eq!(pixel(&off, 39, 9), [32, 23, 17, 255]);
    assert_eq!(pixel(&on, 24, 9), [56, 255, 217, 255]);
    let thumb = pixel(&on, 39, 9);
    assert!(
        (7..=8).contains(&thumb[0])
            && (35..=36).contains(&thumb[1])
            && (30..=31).contains(&thumb[2])
    );
    assert_eq!(thumb[3], 255);
    assert_eq!(pixel(&on, 32, 30), [0, 0, 0, 0]);
}

#[test]
fn surface_switch_parent_opacity_and_sibling_occlusion_are_preserved() {
    let pixels = super::paint_pixels::render(br#"{"id":"root","type":"column","style":{"opacity":0.5},"children":[{"id":"switch","type":"switch","props":{"value":true}}]}"#);
    assert!((127..=128).contains(&pixels[(9 * 64 + 24) * 4 + 3]));
    let pixels = super::paint_pixels::render(br#"{"id":"root","type":"stack","children":[{"id":"switch","type":"switch"},{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert!(pixels.chunks_exact(4).all(|p| p == [255, 0, 0, 255]));
}

#[test]
fn surface_switch_rejects_unimplemented_children_and_hidden_has_no_pixels() {
    let scene = Scene::parse(
        br#"{"id":"switch","type":"switch","children":[{"id":"child","type":"text"}]}"#,
    )
    .unwrap();
    assert!(NativeMeasure::new(&scene, base(), context(), [64., 64.]).is_err());
    let pixels = super::paint_pixels::render(
        br#"{"id":"switch","type":"switch","props":{"visible":false,"value":true}}"#,
    );
    assert!(pixels.iter().all(|v| *v == 0));
}
