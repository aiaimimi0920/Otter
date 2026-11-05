//! 同生产 QuickJS 的字体声明语义；真实 HWND/child/产品由另一个测试 owner 验证。
use super::document_model_tests::collect;
#[test]
fn javascript_typography_engine_reentrant_conversion_uses_original_native_stack() {
    let mut engine = super::Engine::new().unwrap();
    let result = engine.apply(
        super::Command::Mount {
            source: include_str!("../tests/fixtures/javascript-typography-geometry.js").into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Some(crate::surface_viewport::Viewport::new([600, 400], 96).unwrap()),
    );
    if let Err(error) = result {
        engine.context.with(|ctx| {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|object| object.get::<_, String>("stack").ok());
            panic!(
                "Typography trusted fixture: {error:?}: {exception:?}: {stack:?}; geometry={:?}",
                engine.geometry.borrow().trace
            );
        });
    }
}
#[test]
fn javascript_typography_model_shorthand_resets_longhands_and_unitless_inherits() {
    let value = collect(include_str!("../tests/fixtures/javascript-typography.js"));
    let root = &value["scene"];
    assert_eq!(root["style"]["fontSize"], "16px");
    assert_eq!(root["style"]["lineHeight"], "1.5em");
    let first = &root["children"][0]["style"];
    assert_eq!(first["fontSize"], "20px");
    assert_eq!(first["lineHeight"], "1.5em");
    assert_eq!(first["fontWeight"], 700);
    let second = &root["children"][1]["style"];
    assert_eq!(second["fontSize"], "12px");
    assert_eq!(second["lineHeight"], "normal");
    assert_eq!(second["fontWeight"], 600);
    assert_eq!(second["fontFamily"], "\"Consolas\",monospace");
}
#[test]
fn javascript_typography_model_unitless_uses_native_initial_font_and_normal_resets() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
        root.style.lineHeight='1.25';root.innerHTML='<div><span>text</span></div>';
        root.firstChild.style.lineHeight='normal';
    }});"#,
    );
    assert_eq!(value["scene"]["style"]["lineHeight"], "1.25em");
    assert_eq!(
        value["scene"]["children"][0]["children"][0]["style"]["lineHeight"],
        "normal"
    );
}
#[test]
fn javascript_typography_model_uses_captured_intrinsics() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-typography-integrity.js"
    ));
    assert_eq!(value["scene"]["style"]["fontWeight"], 700);
    assert_eq!(value["scene"]["style"]["fontSize"], "18px");
    assert_eq!(value["scene"]["style"]["lineHeight"], "1.25em");
}
#[test]
fn javascript_typography_model_decimal_canonicalization_keeps_native_parser_meaning() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
        root.style.cssText='font-size:0012.px;font-weight:000650;line-height:001.3500';
    }});"#,
    );
    assert_eq!(value["scene"]["style"]["fontSize"], "12px");
    assert_eq!(value["scene"]["style"]["fontWeight"], 650);
    assert_eq!(value["scene"]["style"]["lineHeight"], "1.35em");
}
#[test]
fn javascript_typography_model_cascade_important_and_normal_priority_reset() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
        const sheet=new CSSStyleSheet();sheet.replaceSync(':root{font:700 16px/2 Arial!important;font:12px Consolas;font-size:20px!important}');
        document.adoptedStyleSheets=[sheet];root.style.font='600 14px/normal Segoe UI';
    }});"#,
    );
    assert_eq!(value["scene"]["style"]["fontSize"], "20px");
    assert_eq!(value["scene"]["style"]["fontWeight"], 700);
    assert_eq!(value["scene"]["style"]["lineHeight"], "2em");
    assert_eq!(value["scene"]["style"]["fontFamily"], "\"Arial\"");
}
