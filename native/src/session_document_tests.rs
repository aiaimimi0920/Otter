//! 会话只读边界；不读写用户磁盘，未知业务字段保留在原始字节中。
use super::*;
#[test]
fn original_session_bytes_and_metadata_survive_topology_inspection() {
    let bytes = br#" {
      "documentSchemaVersion":1,"documentRevision":18446744073709551615,
      "stickers":[{"id":"old-a","src":"data:image/png;base64,AAAA","x":-100,"y":20,"w":640,"h":480,
        "annotationState":{"futureTool":[1,2,3]},"params":{"secretField":"unchanged"}}],
      "links":[],"recycleBin":[{"snapshot":{"opaque":true}}],
      "referenceLibrary":[{"snapshot":{"src":"keep"}}],"futureField":{"keep":true}
    } "#;
    let document = decode(bytes).unwrap();
    assert_eq!(document.schema_version, 1);
    assert_eq!(document.revision, u64::MAX);
    assert_eq!(document.workspace.units()[0].as_str(), "old-a");
    let raw_parameters = document.windows[0].params.unwrap().get();
    assert_eq!(raw_parameters, r#"{"secretField":"unchanged"}"#);
    assert!(
        (bytes.as_ptr() as usize..bytes.as_ptr() as usize + bytes.len())
            .contains(&(raw_parameters.as_ptr() as usize))
    );
    assert_eq!(
        document.windows[0].rect(),
        crate::session_window::Rect {
            x: -100.0,
            y: 20.0,
            w: 640.0,
            h: 480.0
        }
    );
    assert_eq!(document.original_json(), bytes);
    assert_eq!(
        document.original_json().as_ptr(),
        bytes.as_ptr(),
        "do not clone the whole session"
    );
}
#[test]
fn legacy_defaults_and_escaped_identifiers_are_accepted() {
    let document =
        decode(br#"{"stickers":[{"id":"\u4e2d\u6587","x":0,"y":0,"w":1,"h":1}],"links":[]}"#)
            .unwrap();
    assert_eq!((document.schema_version, document.revision), (0, 0));
    assert_eq!(document.workspace.units()[0].as_str(), "中文");
}
#[test]
fn future_schema_malformed_fields_and_dangling_links_are_rejected() {
    assert!(matches!(
        decode(br#"{"documentSchemaVersion":2,"stickers":[],"links":[]}"#),
        Err(Error::FutureSchema(2))
    ));
    for bytes in [br#"{"stickers":[],"links":[]} trailing"#.as_slice(),
        br#"{"stickers":[],"links":[],"documentRevision":-1}"#,
        br#"{"stickers":[],"links":[],"groups":null}"#,
        br#"{"stickers":[],"links":[{"id":"x","fromUnitId":"a","fromPortId":"out","toUnitId":"b","toPortId":"in"}]}"#] {
        assert!(decode(bytes).is_err());
    }
}
#[test]
fn record_budget_is_enforced_by_decoder() {
    let stickers: Vec<_> = (0..MAX_UNITS + 1)
        .map(|index| serde_json::json!({"id":format!("s{index}"),"x":0,"y":0,"w":1,"h":1}))
        .collect();
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":stickers,"links":[]})).unwrap();
    assert!(matches!(decode(&bytes), Err(Error::Json(_))));
}
#[test]
fn bad_window_metadata_rejects_whole_candidate_without_rewriting_input() {
    for patch in [
        serde_json::json!({"w":0}),
        serde_json::json!({"opacityNormal":2}),
        serde_json::json!({"savedRect":{"x":0,"y":0,"w":-1,"h":1}}),
    ] {
        let mut value =
            serde_json::json!({"stickers":[{"id":"s","x":0,"y":0,"w":1,"h":1}],"links":[]});
        value["stickers"][0]
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(matches!(decode(&bytes), Err(Error::Window)));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            value
        );
    }
}
