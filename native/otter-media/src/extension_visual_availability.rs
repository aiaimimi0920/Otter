//! Mainline OCR-renderer transitions/unavailable placeholders; project validated data only, without commands or relaxed resource permissions.
use super::*;
use serde_json::json;

pub(super) fn ocr<'a>(catalog: &Snapshot, item: &'a Stored) -> Option<Descriptor<'a>> {
    let a = &item.attachment;
    if a.plugin() != OCR || a.type_id() != OCR_RESULT || !available(catalog, item) {
        return None;
    }
    let payload = a.payload()?;
    // Match mainline: only existing official Surface scenes are usable; never synthesize fake results from fullText.
    if !payload.get("surfaceScene").is_some_and(Value::is_object) {
        return None;
    }
    Some(Descriptor {
        id: "neuro.official/ocr.result-overlay",
        bounds: Bounds::UnitInset(0.),
        command: Some("neuro.official/ocr.copy-block"),
        payload,
        source: false,
        replaces: false,
        generation: 0,
        fallback: true,
    })
}
fn available(catalog: &Snapshot, item: &Stored) -> bool {
    catalog
        .visual_declarations("dataTypes")
        .any(|d| d.plugin == item.attachment.plugin() && d.id == item.attachment.type_id())
}
pub(super) fn scene_unavailable() -> Value {
    json!({"id":"extension-unavailable","type":"text","props":{"text":"扩展视图不可用"}})
}
pub(super) fn unavailable(
    catalog: Option<&Snapshot>,
    store: &Store,
    visuals: &[Visual],
) -> Option<Visual> {
    let children: Vec<_> = store.items().iter().filter(|item| {
        !visuals.iter().any(|v| v.attachment == item.attachment.id())
            && catalog.is_none_or(|c| !available(c, item) || !c.visual_declarations("renderers").take(32)
                .any(|d| descriptor(c, d, item, true).is_some()))
    }).take(8).enumerate().map(|(index, item)| {
        let label = String::from_utf16_lossy(&item.attachment.type_id().encode_utf16().take(64).collect::<Vec<_>>());
        json!({"id":format!("unavailable-{index}"),"type":"text",
            "props":{"text":format!("扩展数据不可用 · {label}"),"selectable":false},
            "layout":{"width":"100%","height":24,"minHeight":24,"paddingLeft":7,"paddingRight":7,
                "paddingTop":4,"paddingBottom":4,"overflowX":"hidden","overflowY":"hidden"},
            "style":{"fontSize":11,"lineHeight":1.25,"whiteSpace":"nowrap","textOverflow":"ellipsis",
                "background":"#0d1117","color":"#42c8de","borderColor":"rgba(66,200,222,0.58)","borderWidth":1,"borderRadius":3}})
    }).collect();
    if children.is_empty() {
        return None;
    }
    Some(Visual {
        plugin: String::new(),
        attachment: String::new(),
        descriptor: "extension-attachment-placeholders".into(),
        bounds: Bounds::UnitInset(6.),
        scene: json!({"id":"extension-attachment-placeholders","type":"column",
            "layout":{"width":"100%","gap":4},"children":children}),
        command: None,
        local_copy: false,
        generation: 0,
        revision: 0,
        placeholder: true,
    })
}
