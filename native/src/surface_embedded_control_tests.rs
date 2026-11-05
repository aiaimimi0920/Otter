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
