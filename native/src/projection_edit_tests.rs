//! 模型边界、独立wire oracle及六操作相关性；不以编码自身生成成功预期。
use super::{fixture::*, *};
use serde_json::{json, Value};
fn parse(v: &Value) -> Result<Document, Error> {
    Document::parse(&serde_json::to_vec(v).unwrap())
}
#[test]
fn six_operations_roundtrip_without_mutating_original_bytes() {
    let common = json!({"projectionId":PROJECTION,"sessionId":SESSION});
    let operations = [
        json!({"operation":"read","projectionId":PROJECTION}),
        json!({"operation":"initialize","expectedDigest":DIGEST,"objects":view()}),
        json!({"operation":"attach"}),
        json!({"operation":"mode","opId":"mode-1","baseModeRevision":1,"mode":"two_way"}),
        json!({"operation":"apply","opId":"apply-1","baseRevision":1,"modeRevision":1,"changes":[{"objectId":"shape-1","value":null}]}),
        json!({"operation":"checkpoint","expectedRevision":1}),
    ];
    for (index, mut v) in operations.into_iter().enumerate() {
        if index != 0 {
            v.as_object_mut()
                .unwrap()
                .extend(common.as_object().unwrap().clone());
        }
        let request = Request::parse(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(request.bytes()).unwrap(), v);
        assert_eq!(Request::parse(request.bytes()).unwrap(), request);
    }
}
#[test]
fn missing_null_unknown_duplicate_and_trailing_fields_are_not_accepted() {
    for text in [
        r#"{"operation":"read"}"#,
        r#"{"operation":"read","projectionId":null}"#,
        r#"{"operation":"read","projectionId":"projection:11111111111111111111111111111111","extra":true}"#,
        r#"{"operation":"read","projectionId":"projection:11111111111111111111111111111111","operation":"read"}"#,
        r#"{"operation":"apply","projectionId":"projection:11111111111111111111111111111111","sessionId":"edit:22222222222222222222222222222222","opId":"x","baseRevision":1,"modeRevision":1,"changes":[{"objectId":"x"}]}"#,
    ] {
        assert!(Request::parse(text.as_bytes()).is_err(), "{text}");
    }
    assert!(json::parse(br##"{"style":{"color":"#fff","color":"#000"}}"##, MAX_BYTES).is_err());
    assert!(json::parse(b"{} {}", MAX_BYTES).is_err());
    assert!(json::parse("[".repeat(130).as_bytes(), MAX_BYTES).is_err());
    assert_eq!(
        json::parse(&vec![b' '; MAX_BYTES + 1], MAX_BYTES),
        Err(Error::Limit)
    );
    let mut doc = document_value();
    doc["objects"]["shape-1"]
        .as_object_mut()
        .unwrap()
        .remove("value");
    assert!(
        parse(&doc).is_err(),
        "missing value is not an implicit tombstone"
    );
}

#[test]
fn unique_scanner_rejects_excessive_container_shapes_before_typed_allocation() {
    let map: serde_json::Map<String, Value> =
        (0..257).map(|n| (format!("k{n}"), Value::Null)).collect();
    assert!(json::parse(&serde_json::to_vec(&map).unwrap(), MAX_BYTES).is_err());
    assert!(json::parse(
        &serde_json::to_vec(&vec![Value::Null; 2049]).unwrap(),
        MAX_BYTES
    )
    .is_err());
    assert!(json::parse(br#"{"a":{"b":1},"c":[true,null,"text",1.5]}"#, MAX_BYTES).is_ok());
}
#[test]
fn document_versions_identity_basis_and_tombstones_are_bounded() {
    let baseline = document();
    assert_eq!(baseline.basis().size(), (32, 16));
    for (key, value) in [
        ("revision", json!(0)),
        ("modeRevision", json!(2)),
        ("checkpointRevision", json!(2)),
        ("receiptCount", json!(257)),
        ("sessionId", json!("edit:bad")),
        ("revision", json!(MAX_REVISION + 1)),
    ] {
        let mut v = document_value();
        v[key] = value;
        assert!(parse(&v).is_err(), "{key}");
    }
    for (key, value) in [
        ("width", json!(0)),
        ("height", json!(8193)),
        ("digest", json!("A".repeat(64))),
    ] {
        let mut v = document_value();
        v["basis"][key] = value;
        assert_eq!(parse(&v), Err(Error::Basis));
    }
    let mut v = document_value();
    v["objects"]["shape-1"]["value"] = Value::Null;
    let tombstone = parse(&v).unwrap();
    assert!(tombstone.live_objects().is_empty());
    assert!(
        tombstone.follows(&baseline).is_err(),
        "same revision cannot change content"
    );
    v["revision"] = 2.into();
    v["objects"]["shape-1"]["revision"] = 2.into();
    assert!(parse(&v).unwrap().follows(&baseline).is_ok());
    v["basis"]["width"] = 33.into();
    assert_eq!(parse(&v).unwrap().follows(&baseline), Err(Error::Basis));
}
#[test]
fn annotation_types_styles_alpha_and_extensions_have_explicit_contract() {
    for kind in ["rect", "round-rect", "ellipse", "triangle", "polygon"] {
        let mut v = annotation("shape-1");
        v["type"] = kind.into();
        assert!(objects::annotation("shape-1", &v));
    }
    for kind in ["line", "polyline", "arrow", "brush", "highlighter"] {
        let v = json!({"id":"x","type":kind,"zIndex":0,"style":{"color":"#fff","width":1},"points":[{"x":0,"y":0},{"x":2,"y":3}]});
        assert!(objects::annotation("x", &v));
    }
    for kind in ["text", "serial"] {
        let v = json!({"id":"x","type":kind,"zIndex":0,"style":{"color":"#fff","width":1},"x":0,"y":0,"text":"中文😀"});
        assert!(objects::annotation("x", &v));
    }
    for color in [
        "#1234",
        "#12345678",
        "rgba(1,2,3,0.5)",
        "rgb(10%,20%,30%)",
        "transparent",
    ] {
        let mut v = annotation("shape-1");
        v["style"]["color"] = color.into();
        assert!(objects::annotation("shape-1", &v), "{color}");
    }
    for color in [
        "currentColor",
        "red",
        "url(http://secret)",
        "rgba(1,2,3,NaN)",
        "#12345",
        "rgb(1%,2,3)",
    ] {
        let mut v = annotation("shape-1");
        v["style"]["color"] = color.into();
        assert!(!objects::annotation("shape-1", &v), "{color}");
    }
    for kind in ["blur", "mosaic", "bezier", "script"] {
        let mut v = annotation("shape-1");
        v["type"] = kind.into();
        assert!(!objects::annotation("shape-1", &v));
    }
    for (key, value) in [
        ("id", json!("other")),
        ("zIndex", Value::Null),
        ("rotation", json!(36001)),
        ("sides", json!(2)),
        ("script", json!("evil")),
    ] {
        let mut v = annotation("shape-1");
        v[key] = value;
        assert!(!objects::annotation("shape-1", &v), "{key}");
    }
}
#[test]
fn request_change_counts_keys_and_object_budgets_are_checked() {
    let make = |changes| {
        Request::prepare(Operation::Apply {
            projection_id: PROJECTION.into(),
            session_id: SESSION.into(),
            op_id: "apply-1".into(),
            base_revision: 1,
            mode_revision: 1,
            changes,
        })
    };
    assert_eq!(make(vec![]), Err(Error::Limit));
    let change = Change {
        object_id: "shape-1".into(),
        value: Value::Null,
    };
    assert_eq!(make(vec![change.clone(), change]), Err(Error::Identity));
    assert_eq!(
        make(
            (0..33)
                .map(|n| Change {
                    object_id: format!("x{n}"),
                    value: Value::Null
                })
                .collect()
        ),
        Err(Error::Limit)
    );
    for id in ["", "__proto__", "constructor", "prototype", "x?y", "x\n"] {
        assert!(!identifier(id));
    }
    let too_many = (0..257)
        .map(|n| {
            let id = format!("x{n}");
            (id.clone(), annotation(&id))
        })
        .collect();
    assert_eq!(objects::validate(&too_many), Err(Error::Limit));
    let huge = json!({"id":"x","type":"text","zIndex":0,"style":{"color":"#fff","width":1},"x":0,"y":0,"text":"😀".repeat(4096)});
    assert!(!objects::annotation("x", &huge));
}
#[test]
fn success_response_must_cross_operation_fences_but_can_include_later_edits() {
    let bytes = serde_json::to_vec(&document_value()).unwrap();
    assert!(mode().response(&bytes, Some(&document())).is_err());
    assert_eq!(
        mode()
            .response(
                &serde_json::to_vec(&mode_document()).unwrap(),
                Some(&document())
            )
            .unwrap(),
        mode_document()
    );
    let apply = Request::prepare(Operation::Apply {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        op_id: "op".into(),
        base_revision: 1,
        mode_revision: 1,
        changes: vec![Change {
            object_id: "x".into(),
            value: Value::Null,
        }],
    })
    .unwrap();
    assert!(apply.response(&bytes, Some(&document())).is_err());
    let mut later = document_value();
    later["revision"] = 3.into();
    assert!(apply
        .response(&serde_json::to_vec(&later).unwrap(), Some(&document()))
        .is_ok());
    later["modeRevision"] = 2.into();
    assert!(apply
        .response(&serde_json::to_vec(&later).unwrap(), Some(&document()))
        .is_err());
    let checkpoint = Request::prepare(Operation::Checkpoint {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        expected_revision: 1,
    })
    .unwrap();
    assert!(checkpoint.response(&bytes, Some(&document())).is_err());
    later["checkpointRevision"] = 2.into();
    assert!(checkpoint
        .response(&serde_json::to_vec(&later).unwrap(), Some(&document()))
        .is_ok());
    let mut wrong = document_value();
    wrong["sessionId"] = format!("edit:{}", "3".repeat(32)).into();
    assert_eq!(
        initialize().response(&serde_json::to_vec(&wrong).unwrap(), None),
        Err(Error::Identity)
    );
    wrong = document_value();
    wrong["basis"]["digest"] = "b".repeat(64).into();
    assert_eq!(
        initialize().response(&serde_json::to_vec(&wrong).unwrap(), None),
        Err(Error::Basis)
    );
}
