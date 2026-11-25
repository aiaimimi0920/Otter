use super::*;
use serde_json::json;
fn sticker(id: &str) -> Value {
    json!({"id":id,"x":0,"y":0,"w":2,"h":1,"src":"own","previewSrc":"preview"})
}
fn resolve(stickers: Vec<Value>, links: Value, catalog: &[Value]) -> Vec<Option<String>> {
    let saved =
        serde_json::to_vec(&json!({"documentRevision":1,"stickers":stickers,"links":links}))
            .unwrap();
    with_sources(&saved, catalog, &AtomicBool::new(false), |_, sources| {
        Ok(sources.iter().map(|s| s.map(str::to_owned)).collect())
    })
    .unwrap()
}
fn link() -> Value {
    json!([{"id":"edge","fromUnitId":"a","toUnitId":"b","fromPortId":"output","toPortId":"image"}])
}
#[test]
fn workflow_sources_prefer_preview_except_with_independent_raster_layer() {
    let a = sticker("a");
    let mut b = sticker("b");
    b["rasterizedAnnotationLayerSrc"] = json!("layer");
    assert_eq!(
        resolve(vec![a, b], json!([]), &[]),
        vec![Some("preview".into()), Some("own".into())]
    );
}
#[test]
fn workflow_sources_honor_upstream_manual_and_propagation_switches() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let mut a = sticker("a");
    a["previewSrc"] = json!("upstream");
    let mut b = sticker("b");
    // 图片参数现在校验内联 Base64；用实际 PNG 保持这个用例专测源的优先级。
    let manual = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(include_bytes!("../assets/branding/32x32.png"))
    );
    b["params"] = json!({"image_path":manual});
    assert_eq!(
        resolve(vec![a.clone(), b.clone()], link(), &[])[1].as_deref(),
        Some("upstream")
    );
    assert_eq!(
        resolve(vec![a.clone(), b.clone()], json!([]), &[])[1].as_deref(),
        Some(manual.as_str())
    );
    for flag in [
        json!({"acceptUpstream":false}),
        json!({"locallyEdited":true}),
    ] {
        b["stickerEditPropagation"] = flag;
        assert_eq!(
            resolve(vec![a.clone(), b.clone()], link(), &[])[1].as_deref(),
            Some("preview")
        );
    }
    b["stickerEditPropagation"] = Value::Null;
    b["params"]["image"] = json!(crate::graph_params::DISABLED);
    assert_eq!(
        resolve(vec![a, b], link(), &[])[1].as_deref(),
        Some("preview")
    );
}
#[test]
fn workflow_sources_reject_invalid_propagation_before_consuming_sources() {
    let mut unit = sticker("a");
    unit["stickerEditPropagation"] = json!({"locallyEdited":"not a boolean"});
    let saved =
        serde_json::to_vec(&json!({"documentRevision":1,"stickers":[unit],"links":[]})).unwrap();
    assert!(matches!(
        with_sources::<()>(&saved, &[], &AtomicBool::new(false), |_, _| panic!(
            "invalid propagation reached consumer"
        )),
        Err(Error::Content)
    ));
}
#[test]
fn workflow_sources_use_formal_art_output_not_visual_preview() {
    let mut a = sticker("a");
    a["type"] = json!("art");
    a["artId"] = json!("test");
    a["outputs"] = json!({"output":{"value":"formal"}});
    let b = sticker("b");
    let catalog = [json!({"id":"test","inputs":[],"outputs":[]})];
    assert_eq!(
        resolve(vec![a.clone(), b.clone()], link(), &catalog)[1].as_deref(),
        Some("formal")
    );
    a["outputs"] = Value::Null;
    assert_eq!(
        resolve(vec![a, b], link(), &catalog)[1].as_deref(),
        Some("preview")
    );
}

#[test]
fn workflow_sources_use_declared_ports_even_when_parameter_port_is_hidden() {
    let a = sticker("a");
    let mut b = sticker("b");
    b["type"] = json!("art");
    b["artId"] = json!("test");
    b["previewSrc"] = Value::Null;
    let mut links = link();
    links[0]["toPortId"] = json!("custom_file");
    let mut capability = json!({"id":"test","inputs":[{"name":"custom_file","type":"any"}],"parameters":[{"id":"custom_file"}]});
    assert_eq!(
        resolve(
            vec![a.clone(), b.clone()],
            links.clone(),
            &[capability.clone()]
        )[1]
        .as_deref(),
        Some("preview")
    );
    capability["inputs"] = json!([]);
    assert_eq!(
        resolve(vec![a, b], links, &[capability])[1].as_deref(),
        Some("own")
    );
}
