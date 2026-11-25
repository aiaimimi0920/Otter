use super::*;
fn window(settings: Value) -> String {
    let mut value = serde_json::json!({"id":"one", "x":0,"y":0,"w":100,"h":80});
    value
        .as_object_mut()
        .unwrap()
        .extend(settings.as_object().unwrap().clone());
    value.to_string()
}
fn read(value: &str) -> Settings {
    Settings::read(&serde_json::from_str(value).unwrap()).unwrap()
}
#[test]
fn baseline_defaults_and_boolean_policy_restore_without_art_authority() {
    assert_eq!(read(&window(serde_json::json!({}))), Settings::default());
    let text = window(serde_json::json!({"hideCollapsedEndpoints":true,
        "executionConfig":{"propagation":{"notifyDownstream":false}}}));
    assert_eq!(
        read(&text),
        Settings {
            hide_collapsed_endpoints: true,
            notify_downstream: false
        }
    );
    let text =
        window(serde_json::json!({"executionConfig":{"propagation":{"notifyDownstream":"false"}}}));
    assert!(read(&text).notify_downstream); // As in mainline Config, strings are not booleans.
    assert!(
        serde_json::from_str::<crate::session_window::Window<'_>>(&window(
            serde_json::json!({"hideCollapsedEndpoints":"true"})
        ))
        .is_err()
    );
}
#[test]
fn sticker_write_preserves_unknown_and_other_execution_fields() {
    let mut fields = serde_json::json!({"executionConfig":{
        "triggerMode":{"paramDriven":false},"custom":[1,2],
        "propagation":{"listenUpstream":false,"custom":"keep","notifyDownstream":true}
    },"unrelated":{"keep":true}})
    .as_object()
    .unwrap()
    .clone();
    let settings = Settings {
        hide_collapsed_endpoints: true,
        notify_downstream: false,
    };
    settings.write(&mut fields, false).unwrap();
    assert_eq!(fields["hideCollapsedEndpoints"], true);
    assert_eq!(
        fields["executionConfig"]["propagation"]["notifyDownstream"],
        false
    );
    assert_eq!(
        fields["executionConfig"]["propagation"]["listenUpstream"],
        false
    );
    assert_eq!(fields["executionConfig"]["propagation"]["custom"], "keep");
    assert_eq!(
        fields["executionConfig"]["triggerMode"]["paramDriven"],
        false
    );
    assert_eq!(
        fields["executionConfig"]["custom"],
        serde_json::json!([1, 2])
    );
    assert_eq!(fields["unrelated"]["keep"], true);
    assert_eq!(read(&window(Value::Object(fields))), settings);
}
#[test]
fn default_sticker_does_not_materialize_config_and_art_keeps_its_owner() {
    let mut fields = Map::new();
    Settings::default().write(&mut fields, false).unwrap();
    assert!(fields.is_empty());
    fields.insert(
        "executionConfig".into(),
        serde_json::json!({"propagation":{"notifyDownstream":false}}),
    );
    let original = fields["executionConfig"].clone();
    Settings::default().write(&mut fields, true).unwrap();
    assert_eq!(fields["executionConfig"], original);
    fields.insert("hideCollapsedEndpoints".into(), true.into());
    Settings::default().write(&mut fields, true).unwrap();
    assert_eq!(fields["hideCollapsedEndpoints"], false);
}
#[test]
fn unrecognized_existing_config_is_preserved_when_policy_has_not_changed() {
    for value in [
        Value::Null,
        serde_json::json!(false),
        serde_json::json!({"propagation":null}),
    ] {
        let mut fields = Map::from_iter([("executionConfig".into(), value.clone())]);
        Settings::default().write(&mut fields, false).unwrap();
        assert_eq!(fields["executionConfig"], value);
    }
}

#[test]
fn explicit_policy_can_initialize_null_but_never_overwrite_unknown_scalar() {
    let settings = Settings {
        notify_downstream: false,
        ..Default::default()
    };
    for value in [Value::Null, serde_json::json!({"propagation":null})] {
        let mut fields = Map::from_iter([("executionConfig".into(), value)]);
        settings.write(&mut fields, false).unwrap();
        assert_eq!(
            fields["executionConfig"]["propagation"]["notifyDownstream"],
            false
        );
    }
    let mut fields = Map::from_iter([("executionConfig".into(), Value::Bool(false))]);
    assert!(settings.write(&mut fields, false).is_err());
    assert_eq!(fields["executionConfig"], false);
    assert_eq!(
        read(&window(serde_json::json!({"hideCollapsedEndpoints":null}))),
        Settings::default()
    );
}
