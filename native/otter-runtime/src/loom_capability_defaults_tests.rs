use super::*;
use serde_json::json;

#[test]
fn schema_port_and_parameter_priority_match_node_factory() {
    let raw = json!({"id":"art", "parameters":[
        {"id":"schema","type":"boolean","default":"0x10"},
        {"id":"port","type":"number","default":" FALSE "},
        {"id":"param","type":"number","default":"1.25"},
        {"id":"any","type":"number","default":"12"},
        {"id":"union","default":"true"}
    ],"inputs":[{"name":"schema","type":"string"},
        {"name":"port","data_type":"boolean","execution_type":"number"},
        {"name":"any"}],"execution":{"inputSchema":{"properties":{
            "schema":{"type":["null","integer"]},
            "union":{"anyOf":[{"type":"null"},{"oneOf":[{"type":"boolean"}]}]}
        }}}});
    let cap = Capability::new(&raw).unwrap();
    let values = cap.initial_parameters().unwrap();
    assert_eq!(
        values
            .iter()
            .map(|(_, v)| v.as_ref().unwrap().as_ref().clone())
            .collect::<Vec<_>>(),
        [
            json!(16.0),
            json!(false),
            json!(1.25),
            json!("12"),
            json!(true)
        ]
    );
}

#[test]
fn blanks_invalid_numbers_and_non_string_values_are_not_rewritten() {
    for original in [
        json!(" "),
        json!("\u{feff}"),
        json!("invalid"),
        json!("Infinity"),
        json!(null),
        json!([1, 2]),
        json!({"x":1}),
        json!(false),
        json!(12),
    ] {
        let raw = json!({"id":"art","parameters":[{"id":"v","type":"number","default":original}]});
        let cap = Capability::new(&raw).unwrap();
        let values = cap.initial_parameters().unwrap();
        assert!(matches!(values[0].1, Some(Cow::Borrowed(_))));
        assert_eq!(values[0].1.as_ref().unwrap().as_ref(), &original);
    }
    for original in ["yes", "1", "off", ""] {
        let raw = json!({"id":"art","parameters":[{"id":"v","type":"boolean","default":original}]});
        let cap = Capability::new(&raw).unwrap();
        let values = cap.initial_parameters().unwrap();
        assert_eq!(values[0].1.as_ref().unwrap().as_ref(), &json!(original));
    }
}

#[test]
fn defaults_do_not_clamp_and_duplicate_last_value_wins_without_secret_leak() {
    let raw = json!({"id":"art","parameters":[
        {"id":"n","widget":"number","default":"100","min":0,"max":10},
        {"id":"missing"}, {"id":"duplicate","default":1},{"id":"duplicate","default":2},
        {"id":"secret","default":"hidden"},{"id":"secret","type":"secret"},
        {"id":"__exec_private","default":"internal"}
    ]});
    let cap = Capability::new(&raw).unwrap();
    let values = cap.initial_parameters().unwrap();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].1.as_ref().unwrap().as_ref(), &json!(100.0));
    assert!(values[1].1.is_none());
    assert_eq!(values[2].1.as_ref().unwrap().as_ref(), &json!(2));
}

#[test]
fn schema_work_is_bounded_instead_of_silently_changing_type_on_exhaustion() {
    let mut schema = json!({"type":"number"});
    for _ in 0..66 {
        schema = json!({"anyOf":[schema]});
    }
    let raw = json!({"id":"art","parameters":[{"id":"v","default":"10"}],
        "execution":{"inputSchema":{"properties":{"v":schema}}}});
    let cap = Capability::new(&raw).unwrap();
    assert_eq!(cap.initial_parameters().unwrap_err(), Error::Capacity);
}
