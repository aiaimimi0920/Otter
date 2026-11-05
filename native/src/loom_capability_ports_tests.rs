use super::*;
#[test]
fn display_policy_uses_only_current_port_declaration_not_old_mapping_or_parameter_value() {
    let raw = serde_json::json!({"id":"visible-test","label":"visible","inputs":[
        {"name":"a","defaultVisible":false},{"name":"same","defaultVisible":true},
        {"name":"legacy"},{"name":"bad","defaultVisible":"false"}],
        "outputs":[{"name":"same","defaultVisible":false}],
        "defaultVisibility":{"a":true,"same":false,"legacy":false,"bad":false},
        "parameters":[{"id":"parameter","default":false}]});
    let cap = Capability::new(&raw).unwrap();
    assert!(!cap.port_default_visible("a", true));
    assert!(cap.port_default_visible("same", true));
    assert!(!cap.port_default_visible("same", false));
    assert!(cap.port_default_visible("legacy", true));
    assert!(cap.port_default_visible("bad", true));
    assert!(cap.port_default_visible("parameter", true));
    assert!(cap.port_default_visible("unknown", false));
}
use serde_json::json;
#[test]
fn port_types_and_metadata_follow_canonical_precedence() {
    let raw = json!({"id":"art","inputs":[
        {"name":" a ","type":" number ","data_type":"image","execution_type":"string", "default":null,
            "label":" A ","required":true,"defaultVisible":false,"exposePort":true,"widget":" slider "},
        {"name":"b","data_type":"boolean","execution_type":"image"},
        {"name":"c","execution_type":"image"},{"name":"d"},{"name":" "}
    ],"outputs":[{"name":"output","data_type":"image"}]});
    let cap = Capability::new(&raw).unwrap();
    let ports: Vec<_> = cap.inputs().collect();
    assert_eq!(
        ports.iter().map(|p| p.kind).collect::<Vec<_>>(),
        ["number", "boolean", "image", "any"]
    );
    assert_eq!(ports[0].name, "a");
    assert_eq!(ports[0].label, "A");
    assert_eq!(ports[0].widget, Some("slider"));
    assert_eq!(ports[0].default, Some(&Value::Null));
    assert_eq!(ports[0].required, Some(true));
    assert_eq!(ports[0].default_visible, Some(false));
    assert!(ports[0].is_image_like());
    assert_eq!(cap.outputs().next().unwrap().kind, "image");
}
#[test]
fn duplicated_scalars_and_reference_images_use_parameter_rows_unless_exposed() {
    let raw = json!({"id":"art","parameters":[{"id":"amount"},{"id":"reference_image"},
        {"id":"input_image"},{"id":"reference2"},{"id":"input"}],"inputs":[
        {"name":"amount","type":"number","exposePort":true},
        {"name":"reference_image"}, {"name":"input_image"},
        {"name":"reference2","type":"IMAGE","exposePort":true},
        {"name":"input","type":"string"}, {"name":"extra","type":"string"}
    ]});
    let cap = Capability::new(&raw).unwrap();
    assert_eq!(
        cap.visible_inputs().map(|p| p.name).collect::<Vec<_>>(),
        ["input_image", "reference2", "extra"]
    );
    assert_eq!(cap.inputs().count(), 6);
}
#[test]
fn empty_input_definitions_are_not_missing_and_name_fallback_is_case_insensitive() {
    let missing = json!({"id":"art"});
    assert!(!Capability::new(&missing).unwrap().has_input_definitions());
    let empty = json!({"id":"art","inputs":[]});
    let cap = Capability::new(&empty).unwrap();
    assert!(cap.has_input_definitions());
    assert_eq!(cap.inputs().count(), 0);
    for name in ["IMAGE", "Source_Image", "reference_image"] {
        let raw = json!({"id":"art","inputs":[{"name":name}]});
        assert!(Capability::new(&raw)
            .unwrap()
            .inputs()
            .next()
            .unwrap()
            .is_image_like());
    }
}
