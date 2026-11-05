//! 对齐主干全局节点/连线合同；不把准备成功当成保存或同步成功。
use super::*;
fn saved() -> Vec<u8> {
    serde_json::to_vec(&json!({"documentSchemaVersion":1,"documentRevision":7,
        "stickers":[
            {"id":"a","type":"art","artId":"art:test","x":-10,"y":2,"w":40,"h":30,
                "params":{"secret":"must-not-copy"},"outputs":{"zero":0,"empty":null},
                "executionConfig":{"enabled":false},"originNodeId":"old-a"},
            {"id":"b","type":"sticker","x":20,"y":30,"w":50,"h":60,"src":"raw-unbaked"}],
        "links":[{"id":"edge","fromUnitId":"a","fromPortId":"zero","toUnitId":"b","toPortId":"input"}],
        "privateRoot":"must-not-copy"
    })).unwrap()
}
fn content(_: &Window<'_>, _: &Map<String, Value>) -> Result<Content, Error> {
    Ok(Content {
        parameters: Map::new(),
        image: Map::new(),
    })
}
#[test]
fn global_snapshot_maps_exact_saved_revision_and_explicit_prepared_content() {
    let snapshot = prepare(&saved(), 7, &AtomicBool::new(false), |window, _| {
        Ok(Content {
            parameters: serde_json::from_value(json!({"safe":true})).unwrap(),
            image: if window.is_art() {
                Map::new()
            } else {
                serde_json::from_value(json!({"src":"baked","rasterizedAnnotationLayerSrc":null}))
                    .unwrap()
            },
        })
    })
    .unwrap();
    assert_eq!(snapshot.revision(), 7);
    let value = snapshot.value();
    assert_eq!(value["documentRevision"], 7);
    assert_eq!(value["nodes"][0]["id"], "a");
    assert_eq!(value["nodes"][0]["type"], "artNode");
    assert_eq!(value["nodes"][0]["data"]["outputs"]["zero"], 0);
    assert_eq!(value["nodes"][0]["data"]["params"], json!({"safe":true}));
    assert_eq!(value["nodes"][1]["data"]["src"], "baked");
    assert_eq!(value["edges"][0]["sourceHandle"], "zero");
    let bytes = snapshot
        .encode_payload(4096, &AtomicBool::new(false))
        .unwrap();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["workflow_id"], "hook-live");
    assert_eq!(payload["snapshot"], *value);
    assert!(!String::from_utf8(bytes).unwrap().contains("must-not-copy"));
}
#[test]
fn revision_cancel_and_invalid_art_stop_before_content_preparation() {
    for revision in [0, 6, 8] {
        assert!(matches!(
            prepare(&saved(), revision, &AtomicBool::new(false), |_, _| panic!(
                "stale content"
            )),
            Err(Error::Revision)
        ));
    }
    assert!(matches!(
        prepare(&saved(), 7, &AtomicBool::new(true), |_, _| panic!(
            "cancelled"
        )),
        Err(Error::Cancelled)
    ));
    let mut value: Value = serde_json::from_slice(&saved()).unwrap();
    value["stickers"][0]["artId"] = " ".into();
    assert!(matches!(
        prepare(
            &serde_json::to_vec(&value).unwrap(),
            7,
            &AtomicBool::new(false),
            content
        ),
        Err(Error::Node)
    ));
}
#[test]
fn content_failure_unknown_image_fields_and_payload_budget_are_not_partial_success() {
    assert!(matches!(
        prepare(&saved(), 7, &AtomicBool::new(false), |_, _| Err(
            Error::Content
        )),
        Err(Error::Content)
    ));
    assert!(matches!(
        prepare(&saved(), 7, &AtomicBool::new(false), |_, _| Ok(Content {
            parameters: Map::new(),
            image: serde_json::from_value(json!({"token":"secret"})).unwrap()
        })),
        Err(Error::Content)
    ));
    let snapshot = prepare(&saved(), 7, &AtomicBool::new(false), content).unwrap();
    assert_eq!(
        snapshot.encode_payload(10, &AtomicBool::new(false)),
        Err(Error::Budget)
    );
    assert_eq!(
        snapshot.encode_payload(4096, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
}
