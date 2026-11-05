//! 协作颜色进入既有Editor和canonical格式后的语义往返；预期常量独立于解析器。
use super::*;
use serde_json::json;

fn input(color: &str, fill: &str) -> Box<RawValue> {
    serde_json::value::to_raw_value(&json!({"serialCounter":0,"extension":"keep",
        "elements":[{"id":"alpha-shape","type":"rect","zIndex":0,"x":8,"y":8,"w":32,"h":32,
            "style":{"width":4,"color":color,"fill":fill,"opacity":0.5,
                "secondaryFill":"rgba(1,2,3,0.2)"}}]}))
    .unwrap()
}
#[test]
fn stroke_fill_and_group_alpha_survive_repeated_save_without_quantization_or_compounding() {
    let mut raw = input("rgba(255,0,0,0.12345679)", "#0000ff80");
    for _ in 0..4 {
        let restored = restore(Some(&raw)).unwrap();
        let stroke = &restored.editor.elements()[0];
        assert_eq!(stroke.color, Color(255, 0, 0));
        assert_eq!(stroke.fill, Some(Color(0, 0, 255)));
        assert_eq!(
            stroke.paint_alpha,
            PaintAlpha::new(0.12345679, 128.0 / 255.0).unwrap()
        );
        assert_eq!(stroke.opacity, 0.5);
        assert_eq!(stroke.annotation_id(), "alpha-shape");
        let bytes =
            crate::session_vector_write::encode(&restored.editor, Some(&raw), |_| unreachable!())
                .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["extension"], "keep");
        assert_eq!(
            value["elements"][0]["style"]["secondaryFill"],
            "rgba(1,2,3,0.2)"
        );
        assert_eq!(
            value["elements"][0]["style"]["color"],
            "rgba(255,0,0,0.12345679)"
        );
        raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    }
}
#[test]
fn removing_or_replacing_fill_never_resurrects_original_color_alpha() {
    let raw = input("#ff000040", "#0000ff80");
    let mut editor = restore(Some(&raw)).unwrap().editor;
    let old = editor.elements()[0].style();
    assert!(editor.restyle(0, old.with_fill(None)));
    let bytes =
        crate::session_vector_write::encode(&editor, Some(&raw), |_| unreachable!()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value["elements"][0]["style"].get("fill").is_none());
    assert_eq!(editor.elements()[0].paint_alpha.stroke(), 64.0 / 255.0);
    assert!(editor.undo());
    assert_eq!(editor.elements()[0].style(), old);
    assert!(editor.restyle(0, old.with_fill(Some(Color(0, 255, 0)))));
    let bytes =
        crate::session_vector_write::encode(&editor, Some(&raw), |_| unreachable!()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["elements"][0]["style"]["fill"], "#00ff00");
}
#[test]
fn transparent_stroke_is_not_opaque_and_malformed_fill_rejects_whole_import() {
    let raw = input("transparent", "rgba(0,0,255,0.5)");
    let editor = restore(Some(&raw)).unwrap().editor;
    assert_eq!(
        editor.elements()[0].paint_alpha,
        PaintAlpha::new(0.0, 0.5).unwrap()
    );
    let mut value: Value = serde_json::from_str(raw.get()).unwrap();
    value["elements"][0]["style"]["fill"] = json!({"url":"private-file"});
    let invalid = serde_json::value::to_raw_value(&value).unwrap();
    assert!(matches!(restore(Some(&invalid)), Err(Error::Style)));
}

#[test]
fn smallest_native_alpha_and_long_legacy_opaque_decimal_can_reopen() {
    let raw = input("#ffffff", "#ffffff");
    let mut editor = restore(Some(&raw)).unwrap().editor;
    let tiny = f32::from_bits(1);
    let style = editor.elements()[0]
        .style()
        .with_paint_alpha(PaintAlpha::new(tiny, tiny).unwrap());
    assert!(editor.restyle(0, style));
    let bytes = crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = restore(Some(&raw)).unwrap();
    assert_eq!(
        restored.editor.elements()[0].paint_alpha,
        PaintAlpha::new(tiny, tiny).unwrap()
    );
    let legacy = input(
        &format!("rgba(255,0,0,1.{})", "0".repeat(80)),
        "transparent",
    );
    assert_eq!(
        restore(Some(&legacy)).unwrap().editor.elements()[0]
            .paint_alpha
            .stroke(),
        1.0
    );
}
