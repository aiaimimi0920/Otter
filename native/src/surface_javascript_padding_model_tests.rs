//! 生产 QuickJS/CSS 的四边语义；不以 scene 字符串代替原生测量或实际产品。
use super::document_model_tests::collect;
#[test]
fn javascript_padding_model_shorthand_cascade_reflection_and_atomic_rejection() {
    let value = collect(include_str!("../tests/fixtures/javascript-padding.js"));
    let root = &value["scene"];
    for (side, expected) in [
        ("Top", "3px"),
        ("Right", "7px"),
        ("Bottom", "11px"),
        ("Left", "13px"),
    ] {
        assert_eq!(root["layout"][format!("padding{side}")], expected);
    }
    assert!(root["layout"].get("padding").is_none());
    assert_eq!(root["children"][0]["layout"]["paddingLeft"], "7px");
    assert!(root["children"][1]["layout"].get("paddingLeft").is_none());
}
#[test]
fn javascript_padding_model_captured_intrinsics_expand_each_edge() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-padding-integrity.js"
    ));
    for (side, expected) in [
        ("Top", "1px"),
        ("Right", "2px"),
        ("Bottom", "3px"),
        ("Left", "4px"),
    ] {
        assert_eq!(value["scene"]["layout"][format!("padding{side}")], expected);
    }
}
#[test]
fn javascript_padding_model_later_shorthand_resets_longhand_and_is_not_inherited() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
        const sheet=new CSSStyleSheet();sheet.replaceSync(':root{box-sizing:border-box;padding-left:9px;padding:1px 2px 3px}');
        document.adoptedStyleSheets=[sheet];root.innerHTML='<div></div>';
    }});"#,
    );
    assert_eq!(value["scene"]["layout"]["paddingLeft"], "2px");
    assert!(value["scene"]["children"][0]["layout"]
        .get("paddingLeft")
        .is_none());
}
#[test]
fn javascript_padding_engine_reentrant_write_and_resize_use_original_native_budgets() {
    let mut engine = super::Engine::new().unwrap();
    let result = engine.apply(
        super::Command::Mount {
            source: include_str!("../tests/fixtures/javascript-padding-geometry.js").into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Some(crate::surface_viewport::Viewport::new([200, 80], 96).unwrap()),
    );
    if let Err(error) = result {
        engine.context.with(|ctx| {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|o| o.get::<_, String>("stack").ok());
            panic!(
                "Padding trusted fixture: {error:?}: {exception:?}: {stack:?}; geometry={:?}",
                engine.geometry.borrow().trace
            );
        });
    }
}
