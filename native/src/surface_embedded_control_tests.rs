//! 控件值/选择标签/滑块位置与绘制次序；真实DirectWrite，不创建或截图子窗口。
use super::*;
use crate::{surface_presentation::Presentation, surface_scene::Scene};
fn presentation(scene: serde_json::Value) -> Presentation {
    let size = [128., 64.];
    let mut p = Presentation::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context(size),
        size,
    )
    .unwrap();
    p.embed_controls().unwrap();
    p
}
#[test]
fn fields_and_select_use_declared_values_and_the_existing_normalization_contract() {
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    for (kind, props, text) in [
        (
            "input",
            serde_json::json!({"value":"current","placeholder":"old"}),
            "current",
        ),
        (
            "textarea",
            serde_json::json!({"value":"line1\nline2"}),
            "line1\nline2",
        ),
        ("number", serde_json::json!({"value":42}), "42"),
        (
            "select",
            serde_json::json!({"value":"b","options":[{"value":"a","label":"Alpha"},{"value":"b","label":"Beta"}]}),
            "Beta",
        ),
    ] {
        let p = presentation(
            serde_json::json!({"id":"field","type":kind,"props":props,"layout":{"width":128,"height":64}}),
        );
        let Paint::Text { layout, .. } = p.embedded_control("field").unwrap() else {
            panic!("text field")
        };
        // 内容oracle是独立字面量，不从Paint的新转换函数产生。
        assert_eq!(layout.text_for_test(), text);
    }
    let p = presentation(
        serde_json::json!({"id":"slider","type":"slider","props":{"min":10,"max":30,"step":4,"value":19},"layout":{"width":128,"height":64}}),
    );
    let Paint::Slider { fraction, .. } = p.embedded_control("slider").unwrap() else {
        panic!("slider")
    };
    assert!((*fraction - 0.4).abs() < 1e-6);
    let p = presentation(
        serde_json::json!({"id":"root","type":"view","layout":{"width":128,"height":64},"children":[
        {"id":"field","type":"input","layout":{"width":32,"height":64,"paddingLeft":"10%","paddingTop":"10vw"},"props":{"value":"x"}}]}),
    );
    let Paint::Text { origin, .. } = p.embedded_control("field").unwrap() else {
        panic!("text field")
    };
    assert!(
        (origin[0] - 12.8).abs() < 0.001,
        "padding uses the 128px containing/viewport width, not the 32px field"
    );
    unsafe {
        windows::Win32::System::Com::CoUninitialize();
    }
}

#[test]
fn no_markers_and_nested_sliders_do_not_create_a_text_engine() {
    let scene = Scene::parse(
        br#"{"id":"root","type":"view","children":[
            {"id":"left","type":"view","children":[
                {"id":"a","type":"slider","props":{"min":0,"max":4,"step":1,"value":1}}]},
            {"id":"right","type":"view","children":[
                {"id":"b","type":"slider","props":{"min":0,"max":4,"step":1,"value":3}}]}]}"#,
    )
    .unwrap();
    let no_text = || panic!("no text engine may be created");
    assert!(prepare_with(&scene, std::iter::empty(), |_| None, no_text)
        .unwrap()
        .is_empty());
    let markers = ["b", "a"].map(|id| Marker {
        id: id.into(),
        ink: crate::surface_button_visual::PaintColor::new(
            scene.root().visible_node(id).unwrap(),
            "color",
            crate::surface_color::Color([0., 0., 0., 1.]),
        ),
        origin: [0., 0.],
        size: [64., 24.],
        clip: Rect::from_box([0., 0.], [64., 24.]),
        inset: 0.0.into(),
    });
    let paints = prepare_with(&scene, markers.iter(), |_| None, no_text).unwrap();
    for (id, expected) in [("a", 0.25), ("b", 0.75)] {
        assert!(matches!(&paints[id], Paint::Slider { fraction, .. } if *fraction == expected));
    }
}
