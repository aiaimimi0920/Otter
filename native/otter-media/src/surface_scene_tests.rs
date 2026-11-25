use super::*;
#[test]
fn loom_optional_maps_accept_null_or_missing_without_losing_tree_data() {
    for explicit_null in [false, true] {
        let mut root = serde_json::json!({"id":"root","type":"column",
            "events":{"click":"activate"},"children":[{"id":"title","type":"text",
            "props":{"text":"股票盯盘"},"layout":null,"style":null,"accessibility":null}]});
        if explicit_null {
            for key in ["props", "layout", "style", "accessibility"] {
                root[key] = Value::Null;
            }
        }
        let scene = Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap();
        assert_eq!(scene.node_count(), 2);
        let root = scene.root();
        assert!(root.props().is_empty());
        assert!(root.layout().is_empty());
        assert!(root.style().is_empty());
        assert!(root.accessibility().is_empty());
        assert_eq!(root.events()["click"], "activate");
        assert_eq!(root.children()[0].props()["text"], "股票盯盘");
    }
}
#[test]
fn loom_optional_maps_preserve_object_values() {
    let scene = Scene::parse(br##"{"id":"title","type":"text","props":{"text":"kept"},"layout":{"width":120},"style":{"color":"#fff"},"accessibility":{"label":"Title"}}"##).unwrap();
    assert_eq!(scene.root().props()["text"], "kept");
    assert_eq!(scene.root().layout()["width"], 120);
    assert_eq!(scene.root().style()["color"], "#fff");
    assert_eq!(scene.root().accessibility()["label"], "Title");
}
#[test]
fn loom_optional_maps_reject_non_object_non_null_values() {
    for key in ["props", "layout", "style", "accessibility"] {
        for value in [
            serde_json::json!([]),
            serde_json::json!([{}]),
            serde_json::json!(true),
            serde_json::json!(1),
            serde_json::json!("{}"),
        ] {
            let mut root = serde_json::json!({"id":"root","type":"text"});
            root[key] = value;
            assert_eq!(
                Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap_err(),
                Error::Json,
                "accepted invalid {key}"
            );
        }
    }
}
#[test]
fn loom_optional_maps_do_not_relax_events_or_children() {
    for (key, value) in [
        ("events", Value::Null),
        ("events", serde_json::json!([])),
        ("events", serde_json::json!({"click":null})),
        ("children", Value::Null),
        ("children", serde_json::json!({})),
        ("children", serde_json::json!([null])),
    ] {
        let mut root = serde_json::json!({"id":"root","type":"text",
            "props":null,"layout":null,"style":null,"accessibility":null});
        root[key] = value;
        assert_eq!(
            Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap_err(),
            Error::Json,
            "accepted invalid {key}"
        );
    }
}
#[test]
fn loom_optional_maps_keep_property_and_event_limits() {
    for key in ["props", "layout", "style", "accessibility", "events"] {
        let limit = if key == "events" { 32 } else { 128 };
        let mut root = serde_json::json!({"id":"root","type":"text",
            "props":null,"layout":null,"style":null,"accessibility":null});
        root[key] = Value::Object(
            (0..limit)
                .map(|i| (format!("p{i}"), serde_json::json!("action")))
                .collect(),
        );
        assert!(
            Scene::parse(&serde_json::to_vec(&root).unwrap()).is_ok(),
            "{key}"
        );
        root[key]["overflow"] = serde_json::json!("action");
        assert_eq!(
            Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap_err(),
            Error::Limit,
            "limit bypass for {key}"
        );
    }
}
#[test]
fn inline_image_unavailable_keeps_node_and_alt_without_resource_fallback() {
    let raw = br#"{"id":"root","type":"column","children":[{"id":"inline","type":"image","props":{"src":"data:image/png,bad","alt":"unavailable"}},{"id":"resource","type":"image","props":{"resourceId":"missing","src":"data:image/png,bad"}},{"id":"remote","type":"image","props":{"src":"https://example.invalid/a.png"}},{"id":"hidden","type":"image","props":{"visible":false,"src":"data:image/png,bad"}}]}"#;
    let scene = Scene::parse(raw)
        .unwrap()
        .with_image_availability(|_| false);
    assert_eq!(
        scene
            .root()
            .visible_children()
            .map(Node::id)
            .collect::<Vec<_>>(),
        ["inline"]
    );
    assert_eq!(scene.root().children()[0].props()["alt"], "unavailable");
}
#[test]
fn unresolved_image_projection_preserves_source_and_explicit_visibility() {
    let raw = br#"{"id":"root","type":"column","children":[{"id":"pending","type":"image","props":{"resourceId":"resource:one","src":"data:image/png;base64,source"}},{"id":"hidden","type":"image","props":{"visible":false}},{"id":"text","type":"text","props":{"text":"kept"}}]}"#;
    let scene = Scene::parse(raw).unwrap();
    assert_eq!(scene.root().visible_children().count(), 2);
    let scene = scene.with_image_availability(|_| false);
    assert_eq!(scene.node_count(), 4);
    assert_eq!(
        scene
            .root()
            .visible_children()
            .map(Node::id)
            .collect::<Vec<_>>(),
        ["text"]
    );
    assert_eq!(
        scene.root().children()[0].props()["resourceId"],
        "resource:one"
    );
    assert_eq!(
        scene.root().children()[0].props()["src"],
        "data:image/png;base64,source"
    );
    let scene = scene.with_image_availability(|_| true);
    assert_eq!(
        scene
            .root()
            .visible_children()
            .map(Node::id)
            .collect::<Vec<_>>(),
        ["pending", "text"]
    );
    assert!(!scene.root().children()[1].visible());
}
#[test]
fn unresolved_root_image_projection_cannot_be_set_by_protocol_input() {
    let scene = Scene::parse(
        br#"{"id":"image","type":"image","image_unresolved":true,"image_placeholder":true}"#,
    )
    .unwrap();
    assert!(!scene.root().image_placeholder());
    assert!(scene.root().visible());
    let scene = scene.with_image_availability(|_| false);
    assert!(!scene.root().visible());
    assert!(scene.with_image_availability(|_| true).root().visible());
}
#[test]
fn main_declarative_node_vocabulary_and_data_are_preserved() {
    let kinds = [
        "view", "row", "column", "stack", "scroll", "text", "image", "icon", "button", "input",
        "textarea", "number", "slider", "switch", "select", "progress", "divider", "spacer",
    ];
    for kind in kinds {
        let bytes = serde_json::to_vec(&serde_json::json!({"id":"root", "type":kind,
            "props":{"text":"原生场景"}, "style":{"color":"#fff"}, "events":{"click":"action:1"}}))
        .unwrap();
        let scene = Scene::parse(&bytes).unwrap();
        assert_eq!(scene.node_count(), 1);
        assert_eq!(scene.root().props()["text"], "原生场景");
        assert_eq!(scene.root().events()["click"], "action:1");
    }
}
#[test]
fn invalid_tree_does_not_produce_partial_scene() {
    for (value, expected) in [
        (serde_json::json!({"id":"x", "type":"script"}), Error::Json),
        (serde_json::json!({"id":"", "type":"text"}), Error::Identity),
        (
            serde_json::json!({"id":"x", "type":"view", "children":[{"id":"x", "type":"text"}]}),
            Error::Duplicate,
        ),
        (
            serde_json::json!({"id":"x", "type":"text", "props":[]}),
            Error::Json,
        ),
        (
            serde_json::json!({"id":"x", "type":"text", "events":{"click":"bad\ncommand"}}),
            Error::Identity,
        ),
    ] {
        assert_eq!(
            Scene::parse(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            expected
        );
    }
}
#[test]
fn scene_depth_count_and_bytes_are_bounded() {
    let mut root = serde_json::json!({"id":"leaf", "type":"text"});
    for index in 1..MAX_DEPTH {
        root = serde_json::json!({"id":format!("n{index}"), "type":"view", "children":[root]});
    }
    assert_eq!(
        Scene::parse(&serde_json::to_vec(&root).unwrap())
            .unwrap()
            .node_count(),
        MAX_DEPTH
    );
    root = serde_json::json!({"id":"outer", "type":"view", "children":[root]});
    assert_eq!(
        Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap_err(),
        Error::Limit
    );
    let children: Vec<_> = (0..MAX_NODES)
        .map(|i| serde_json::json!({"id":format!("c{i}"), "type":"text"}))
        .collect();
    let root = serde_json::json!({"id":"root", "type":"view", "children":children});
    assert_eq!(
        Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        Scene::parse(&vec![b' '; MAX_BYTES + 1]).unwrap_err(),
        Error::Limit
    );
}
