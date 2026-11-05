//! 内容策略使用真实 Snapshot 构建入口，不读取图片或发送网络请求。
use super::*;
use serde_json::json;
fn saved(mut fields: Value) -> Vec<u8> {
    let base = fields.as_object_mut().unwrap();
    for (key, value) in [
        ("id", json!("node")),
        ("x", json!(0)),
        ("y", json!(0)),
        ("w", json!(10)),
        ("h", json!(20)),
    ] {
        base.entry(key).or_insert(value);
    }
    serde_json::to_vec(&json!({"documentRevision":2,"stickers":[fields],"links":[]})).unwrap()
}
#[test]
fn matching_capability_filters_secret_and_internal_controls_without_materializing_defaults() {
    let catalog = vec![
        json!({"id":"art","parameters":[{"id":"secret","secret":true},{"id":"missing","default":5},{"id":"zero"}]}),
    ];
    let bytes = saved(
        json!({"type":"art","artId":"art","params":{"secret":"hidden","__exec_mode":"hidden","zero":0,"legacy":false},"src":"original"}),
    );
    let snapshot = prepare(&bytes, 2, &catalog, &AtomicBool::new(false), |_, _, _| {
        panic!("art must not bake")
    })
    .unwrap();
    let params = &snapshot.value()["nodes"][0]["data"]["params"];
    assert_eq!(params, &json!({"zero":0,"legacy":false}));
    assert!(matches!(
        prepare(&bytes, 2, &[], &AtomicBool::new(false), |_, _, _| panic!()),
        Err(Error::Content)
    ));
    let duplicate = vec![catalog[0].clone(), catalog[0].clone()];
    assert!(matches!(
        prepare(
            &bytes,
            2,
            &duplicate,
            &AtomicBool::new(false),
            |_, _, _| panic!()
        ),
        Err(Error::Content)
    ));
}
#[test]
fn edited_minified_sticker_requires_bake_and_never_falls_back_to_unedited_source() {
    let bytes = saved(
        json!({"type":"sticker","src":"unbaked","filePath":"C:/private.png",
        "minified":true,"savedRect":{"x":0,"y":0,"w":100,"h":80},"opacityNormal":0.5}),
    );
    let mut calls = 0;
    let snapshot = prepare(&bytes, 2, &[], &AtomicBool::new(false), |_, _, size| {
        calls += 1;
        assert_eq!(size, (100.0, 80.0));
        Ok("data:image/png;base64,baked".into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    let data = &snapshot.value()["nodes"][0]["data"];
    assert_eq!(data["src"], "data:image/png;base64,baked");
    assert!(data.get("filePath").is_none());
    assert!(data["rasterizedAnnotationLayerSrc"].is_null());
    assert!(matches!(
        prepare(&bytes, 2, &[], &AtomicBool::new(false), |_, _, _| Err(
            Error::Content
        )),
        Err(Error::Content)
    ));
}
#[test]
fn direct_file_backed_preview_is_omitted_and_edit_triggers_are_explicit() {
    let bytes = saved(
        json!({"type":"sticker","src":"base","previewSrc":"preview","filePath":"C:/image.png"}),
    );
    let snapshot = prepare(&bytes, 2, &[], &AtomicBool::new(false), |_, _, _| panic!()).unwrap();
    assert!(snapshot.value()["nodes"][0]["data"]
        .get("previewSrc")
        .is_none());
    for edit in [
        json!({"cropRect":{}}),
        json!({"rotation":90}),
        json!({"flippedX":true}),
        json!({"borderWidth":1}),
        json!({"beautify":{"enabled":true}}),
        json!({"contentEraseStrokes":[{}]}),
    ] {
        let bytes = saved(json!({"type":"sticker","imageEditState":edit}));
        assert!(matches!(
            prepare(&bytes, 2, &[], &AtomicBool::new(false), |_, _, _| Err(
                Error::Content
            )),
            Err(Error::Content)
        ));
    }
}
