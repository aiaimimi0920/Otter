use super::*;
use serde_json::json;

#[test]
fn enabled_defaults_to_true_and_labels_follow_baseline_normalization() {
    for (value, expected) in [
        (json!({"id":" a "}), true),
        (json!({"id":"a", "enabled":false}), false),
        (json!({"id":"a", "enabled":true}), true),
        (json!({"id":"a", "enabled":null}), true),
        (json!({"id":"a", "enabled":"false"}), true),
    ] {
        let capability = Capability::new(&value).unwrap();
        assert_eq!(capability.enabled, expected);
        assert_eq!(capability.id, "a");
        assert_eq!(capability.label, "a");
    }
    assert!(Capability::new(&json!({"id":" "})).is_none());
    assert!(Capability::new(&json!([])).is_none());
}

#[test]
fn defaults_preserve_missing_null_and_borrow_without_copying() {
    let raw = json!({"id":"art", "defaults":{"a":42,"b":23,"c":false}, "parameters":[
        {"id":"a","default":null}, {"id":"b"}, {"id":"c"}, {"id":"d"},
        {"id":"e","default":{"items":[1,2,3]}}, {"id":" "}, {"label":"invalid"}
    ]});
    let capability = Capability::new(&raw).unwrap();
    let params: Vec<_> = capability.parameters().collect();
    assert_eq!(params.len(), 5);
    assert_eq!(params[0].default, Some(&Value::Null));
    assert_eq!(params[1].default, Some(&json!(23)));
    assert_eq!(params[2].default, Some(&json!(false)));
    assert!(params[3].default.is_none());
    assert!(std::ptr::eq(
        params[4].default.unwrap(),
        &raw["parameters"][4]["default"]
    ));
}

#[test]
fn secret_aliases_and_duplicate_declarations_never_expose_defaults() {
    let raw = json!({"id":"art","defaults":{"a":"private","b":"private","c":"private"},
    "parameters":[
        {"id":"a","type":" SECRET ","default":"private"},
        {"id":"b","data_type":"Secret"},
        {"id":"c","secret":true},
        {"id":"a","default":"also private"}
    ]});
    let capability = Capability::new(&raw).unwrap();
    for param in capability.parameters() {
        assert!(param.secret);
        assert!(param.default.is_none());
    }
}

#[test]
fn widgets_options_and_metadata_match_main_normalization() {
    let raw = json!({"id":"art","parameters":[
        {"id":"explicit","widget":" SLIDER ","type":"string","min":0,"max":10,"step":0.5},
        {"id":"select","options":["a",5,false,{"value":"b","label":" B "},null,{},[]]},
        {"id":"bool","data_type":"bool"}, {"id":"int","type":"integer"},
        {"id":"json","type":"object","group":" advanced ","required":true},
        {"id":"path","type":"directory"}, {"id":"image","type":"image/png"},
        {"id":"fallback","options":[null,{}]},
        {"id":"text","widget":"textarea","multiline":false,"disabled":true}
    ]});
    let capability = Capability::new(&raw).unwrap();
    let params: Vec<_> = capability.parameters().collect();
    assert_eq!(
        params.iter().map(|p| p.widget.as_str()).collect::<Vec<_>>(),
        [
            "slider",
            "select",
            "checkbox",
            "number",
            "textarea",
            "path",
            "image_link",
            "text",
            "textarea"
        ]
    );
    assert_eq!(
        (params[0].min, params[0].max, params[0].step),
        (Some(0.0), Some(10.0), Some(0.5))
    );
    let options: Vec<_> = params[1].options().collect();
    assert_eq!(
        options.iter().map(|o| o.label.as_ref()).collect::<Vec<_>>(),
        ["a", "5", "false", "B"]
    );
    assert_eq!(options[2].value, &json!(false));
    assert!(params[4].multiline);
    assert_eq!(params[4].group, Some("advanced"));
    assert_eq!(params[4].required, Some(true));
    assert!(!params[8].multiline);
    assert_eq!(params[8].disabled, Some(true));
}
