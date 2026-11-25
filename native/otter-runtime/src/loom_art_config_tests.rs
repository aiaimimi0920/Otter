use super::*;
use crate::{loom_capability::Capability, loom_parameter_state::Parameters};
use serde_json::{json, Value};
#[test]
fn native_shader_abi_is_host_owned_and_cannot_collide_with_image_or_disabled_parameter() {
    let manual = [("shader_abi", Output::Image(crate::graph_params::DISABLED))];
    let resolved = crate::graph_params::Params::new(&[], &manual).unwrap();
    let config = Configuration::native_shader_from_resolved("art", &resolved, &[]).unwrap();
    let value: Value = serde_json::from_str(&format!("{{\"art\":0{}}}", config.tail)).unwrap();
    assert_eq!(value["parameters"]["shader_abi"], "otter.hlsl.v1");
    assert_eq!(value["parameters"]["output_mode"], "shader");
    assert_eq!(value["disabledParameters"], json!([]));
    assert!(matches!(
        Configuration::native_shader_from_resolved("art", &resolved, &["shader_abi"]),
        Err(Error::InvalidValue)
    ));
}
#[test]
fn shader_and_normal_image_sources_replace_same_name_parameters_without_wire_collision() {
    use crate::graph_params::Params;
    let raw = json!({"id":"art","parameters":[{"id":"overlay","default":"local-unit"},{"id":"amount","default":2}]});
    let parameters = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
    let manual = [
        ("overlay", Output::Image("local-unit")),
        ("amount", Output::Number(2.0)),
    ];
    let resolved = Params::new(&[], &manual).unwrap();
    for config in [
        Configuration::from_parameters(&parameters, &["overlay"]).unwrap(),
        Configuration::shader_from_parameters(&parameters, &["overlay"]).unwrap(),
        Configuration::from_resolved("art", &resolved, &["overlay"]).unwrap(),
        Configuration::shader_from_resolved("art", &resolved, &["overlay"]).unwrap(),
    ] {
        let wire = execute_config(
            &UnitId::parse("unit").unwrap(),
            &Request {
                id: UnitId::parse("request").unwrap(),
                generation: 1,
            },
            &config,
            &[(
                "overlay",
                Input::InlinePng {
                    base64: "AQID",
                    width: 1,
                    height: 1,
                },
            )],
            OutputTransport::Websocket,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&wire).unwrap();
        assert!(value["params"]["parameters"].get("overlay").is_none());
        assert_eq!(value["params"]["parameters"]["amount"], 2.0);
        assert_eq!(value["params"]["inputs"]["overlay"]["dataBase64"], "AQID");
    }
}
#[test]
fn shader_modes_reject_real_image_name_collisions_even_without_declared_parameters() {
    use crate::graph_params::Params;
    for declared in [false, true] {
        let definitions = if declared {
            json!([{"id":"mode","default":"__DISABLED__"},{"id":"output_mode","default":"cpu"}])
        } else {
            json!([])
        };
        let raw = json!({"id":"art","parameters":definitions});
        let parameters = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
        let manual = if declared {
            vec![
                ("mode", Output::Image(crate::graph_params::DISABLED)),
                ("output_mode", Output::Image("cpu")),
            ]
        } else {
            vec![]
        };
        let resolved = Params::new(&[], &manual).unwrap();
        for ports in [
            vec!["mode"],
            vec!["output_mode"],
            vec!["mode", "output_mode"],
        ] {
            assert!(
                matches!(
                    Configuration::shader_from_parameters(&parameters, &ports),
                    Err(Error::InvalidValue)
                ),
                "{declared}: {ports:?}"
            );
            assert!(
                matches!(
                    Configuration::shader_from_resolved("art", &resolved, &ports),
                    Err(Error::InvalidValue)
                ),
                "{declared}: {ports:?}"
            );
        }
    }
}
#[test]
fn ordinary_art_keeps_mode_named_images_and_shader_keeps_nonconflicting_inputs() {
    use crate::graph_params::Params;
    let raw = json!({"id":"art","parameters":[{"id":"mode","default":"cpu"},{"id":"output_mode","default":"png"}]});
    let parameters = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
    let manual = [
        ("mode", Output::Image("cpu")),
        ("output_mode", Output::Image("png")),
    ];
    let resolved = Params::new(&[], &manual).unwrap();
    for ports in [
        vec!["mode"],
        vec!["output_mode"],
        vec!["mode", "output_mode"],
    ] {
        for config in [
            Configuration::from_parameters(&parameters, &ports).unwrap(),
            Configuration::from_resolved("art", &resolved, &ports).unwrap(),
        ] {
            let wire = configured_wire(&config, &ports);
            for name in &ports {
                assert!(wire["params"]["parameters"].get(name).is_none());
                assert_eq!(wire["params"]["inputs"][name]["dataBase64"], "AQID");
            }
            assert_eq!(
                wire["params"]["inputs"].as_object().unwrap().len(),
                ports.len()
            );
        }
    }
    for config in [
        Configuration::shader_from_parameters(&parameters, &["input", "reference"]).unwrap(),
        Configuration::shader_from_resolved("art", &resolved, &["input", "reference"]).unwrap(),
    ] {
        let wire = configured_wire(&config, &["input", "reference"]);
        assert_eq!(
            wire["params"]["parameters"],
            json!({"mode":"shader","output_mode":"shader"})
        );
        assert_eq!(wire["params"]["disabledParameters"], json!([]));
        assert_eq!(wire["params"]["inputs"].as_object().unwrap().len(), 2);
    }
}
fn configured_wire(config: &Configuration, names: &[&str]) -> Value {
    let inputs: Vec<_> = names
        .iter()
        .map(|name| {
            (
                *name,
                Input::InlinePng {
                    base64: "AQID",
                    width: 1,
                    height: 1,
                },
            )
        })
        .collect();
    serde_json::from_str(
        &execute_config(
            &UnitId::parse("unit").unwrap(),
            &Request {
                id: UnitId::parse("request").unwrap(),
                generation: 1,
            },
            config,
            &inputs,
            OutputTransport::Websocket,
        )
        .unwrap(),
    )
    .unwrap()
}
fn parameters() -> Parameters {
    let raw = json!({"id":"art-中文", "parameters":[
        {"id":"number", "type":"number","default":"12"},
        {"id":"flag","default":false}, {"id":"missing"},
        {"id":"null","default":null}, {"id":"disabled","default":"__DISABLED__"},
        {"id":"object","default":{"array":["中文\n",1,true,null]}},
        {"id":"secret","type":"secret","default":"never send"},
        {"id":"__exec_internal","default":"never send"}
    ]});
    Parameters::new(&Capability::new(&raw).unwrap()).unwrap()
}
#[test]
fn edited_parameters_create_immutable_formal_wire_without_secret_or_disabled_values() {
    let mut parameters = parameters();
    parameters.set("number", Some(json!(25))).unwrap();
    let configuration = Configuration::from_parameters(&parameters, &[]).unwrap();
    parameters.set("number", Some(json!(99))).unwrap();
    drop(parameters);
    let unit = UnitId::parse("unit").unwrap();
    let mut registry = Requests::default();
    let outgoing = stage_config(
        &mut registry,
        &unit,
        UnitId::parse("request").unwrap(),
        &configuration,
        &[],
        OutputTransport::Websocket,
    )
    .unwrap();
    let wire: Value = serde_json::from_str(&outgoing.execute).unwrap();
    assert_eq!(wire["method"], "loom.hook.art.execute");
    assert_eq!(wire["params"]["artId"], "art-中文");
    assert_eq!(
        wire["params"]["parameters"],
        json!({
            "number":25,"flag":false,"null":null,"object":{"array":["中文\n",1,true,null]}
        })
    );
    assert_eq!(wire["params"]["disabledParameters"], json!(["disabled"]));
    assert!(!outgoing.execute.contains("never send"));
    assert!(registry.is_latest(&unit, &outgoing.request));
}
#[test]
fn owned_parameters_use_same_input_contract_and_failed_stage_preserves_old_request() {
    let parameters = parameters();
    for ports in [vec![""], vec!["duplicate", "duplicate"], vec!["bad\nport"]] {
        assert!(Configuration::from_parameters(&parameters, &ports).is_err());
    }
    let names: Vec<_> = (0..33).map(|i| format!("input-{i}")).collect();
    let ports: Vec<_> = names.iter().map(String::as_str).collect();
    assert!(Configuration::from_parameters(&parameters, &ports).is_err());
    let unit = UnitId::parse("unit").unwrap();
    let mut registry = Requests::default();
    let first = Configuration::from_parameters(&parameters, &[]).unwrap();
    let old = stage_config(
        &mut registry,
        &unit,
        UnitId::parse("old").unwrap(),
        &first,
        &[],
        OutputTransport::Websocket,
    )
    .unwrap();
    let image_config = Configuration::from_parameters(&parameters, &["image"]).unwrap();
    assert!(stage_config(
        &mut registry,
        &unit,
        UnitId::parse("new").unwrap(),
        &image_config,
        &[],
        OutputTransport::Websocket
    )
    .is_err());
    assert!(registry.is_latest(&unit, &old.request));
}
#[test]
fn shader_configuration_forces_modes_once_without_secret_or_contradictory_disabled_mode() {
    let raw = json!({"id":"art","parameters":[{"id":"mode","default":"__DISABLED__"},{"id":"output_mode","default":"cpu"},{"id":"secret","secret":true,"default":"never send"}]});
    let parameters = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
    let config = Configuration::shader_from_parameters(&parameters, &[]).unwrap();
    let wire = execute_config(
        &UnitId::parse("unit").unwrap(),
        &crate::graph_requests::Request {
            id: UnitId::parse("request").unwrap(),
            generation: 1,
        },
        &config,
        &[],
        OutputTransport::Websocket,
    )
    .unwrap();
    assert_eq!(wire.matches("\"mode\":").count(), 1);
    assert_eq!(wire.matches("\"output_mode\":").count(), 1);
    assert!(!wire.contains("never send"));
    let value: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(
        value["params"]["parameters"],
        json!({"mode":"shader","output_mode":"shader"})
    );
    assert_eq!(value["params"]["disabledParameters"], json!([]));
}
#[test]
fn shader_configuration_resolved_snapshot_forces_mode_without_overriding_linked_scalars() {
    use crate::{
        graph_image::Output,
        graph_param_coercion::Widget,
        graph_params::{Param, Params},
    };
    let declared = [Param {
        id: "secret",
        default: Some(Output::Image("never send")),
        secret: true,
        widget: Widget {
            name: "",
            min: None,
            max: None,
        },
    }];
    let manual = [
        ("mode", Output::Image(crate::graph_params::DISABLED)),
        ("output_mode", Output::Image("cpu")),
        ("gain", Output::Number(0.5)),
        ("disabled", Output::Image(crate::graph_params::DISABLED)),
    ];
    let resolved = Params::new(&declared, &manual).unwrap();
    let config = Configuration::shader_from_resolved("shader", &resolved, &[]).unwrap();
    let wire = execute_config(
        &UnitId::parse("unit").unwrap(),
        &crate::graph_requests::Request {
            id: UnitId::parse("request").unwrap(),
            generation: 1,
        },
        &config,
        &[],
        OutputTransport::Websocket,
    )
    .unwrap();
    assert!(!wire.contains("never send"));
    assert_eq!(wire.matches("\"mode\":").count(), 1);
    let value: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(
        value["params"]["parameters"],
        json!({"mode":"shader","output_mode":"shader","gain":0.5})
    );
    assert_eq!(value["params"]["disabledParameters"], json!(["disabled"]));
}
