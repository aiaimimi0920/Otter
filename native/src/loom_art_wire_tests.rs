//! 外部 Loom method/params 合同；夹具不打开共享内存或发送真实 WebSocket。
use super::*;
use crate::{
    graph_execution_config::{Config, Runtime, Trigger},
    graph_image::{DispatchOptions, Graph, Kind, Node, Port, Preparation},
};
fn fixture(test: impl FnOnce(&UnitId, &mut Prepared<'_>)) {
    let unit = UnitId::parse("unit").unwrap();
    let nodes = [Node {
        id: &unit,
        kind: Kind::Art,
        inputs: &[],
        outputs: &[],
        capability_inputs: Some(&[Port {
            name: "image",
            kind: "image",
        }]),
        capability_outputs: &[],
        values: &[],
        source: None,
        preview: None,
        manual_image: None,
        input_disabled: false,
        accept_upstream: true,
        locally_edited: false,
        pending: false,
    }];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let manual = [
        ("image", Output::Image("source-ref")),
        ("amount", Output::Number(2.0)),
    ];
    let options = DispatchOptions {
        config: Config::default(),
        runtime: Runtime::default(),
        trigger: Trigger::Manual,
    };
    let Preparation::Ready(mut prepared) = graph
        .prepare_dispatch(&unit, &[], &manual, options)
        .unwrap()
    else {
        panic!("prepare");
    };
    test(&unit, &mut prepared);
}
fn request() -> Request {
    Request {
        id: UnitId::parse("request").unwrap(),
        generation: 2,
    }
}
#[test]
fn owned_configuration_outlives_graph_and_preserves_wire_and_atomic_registration() {
    let mut snapshot = None;
    let mut expected = String::new();
    fixture(|unit, prepared| {
        prepared.disabled.push("optional");
        let config = Configuration::capture("art\"中文", prepared).unwrap();
        let inputs = [(
            "image",
            Input::InlinePng {
                base64: "AQID",
                width: 1,
                height: 1,
            },
        )];
        expected = execute(
            unit,
            "art\"中文",
            &request(),
            prepared,
            &inputs,
            OutputTransport::Websocket,
        )
        .unwrap();
        snapshot = Some(config);
    });
    // 原 Graph/Params/手动字符串借用已离开作用域，快照仍能独立编码。
    let config = snapshot.unwrap();
    let unit = UnitId::parse("unit").unwrap();
    let inputs = [(
        "image",
        Input::InlinePng {
            base64: "AQID",
            width: 1,
            height: 1,
        },
    )];
    assert_eq!(
        execute_config(
            &unit,
            &request(),
            &config,
            &inputs,
            OutputTransport::Websocket
        )
        .unwrap(),
        expected
    );
    let value: serde_json::Value = serde_json::from_str(&expected).unwrap();
    assert_eq!(value["params"]["parameters"]["amount"], 2.0);
    assert!(value["params"]["parameters"].get("image").is_none());
    assert_eq!(
        value["params"]["disabledParameters"],
        serde_json::json!(["optional"])
    );
    let mut registry = Requests::default();
    let old = registry
        .begin(unit.clone(), UnitId::parse("old").unwrap())
        .unwrap();
    assert!(stage_config(
        &mut registry,
        &unit,
        UnitId::parse("new").unwrap(),
        &config,
        &[],
        OutputTransport::Websocket
    )
    .is_err());
    assert!(registry.is_latest(&unit, &old));
    let staged = stage_config(
        &mut registry,
        &unit,
        UnitId::parse("new").unwrap(),
        &config,
        &inputs,
        OutputTransport::Websocket,
    )
    .unwrap();
    assert_eq!(staged.request.generation, 2);
    assert!(staged.cancel.is_some());
}
#[test]
fn external_staging_is_atomic_and_uses_external_cancel_schema() {
    fixture(|unit, prepared| {
        let mut registry = Requests::default();
        let old = registry
            .begin(unit.clone(), UnitId::parse("old").unwrap())
            .unwrap();
        registry
            .mark_preview(unit, &old, "previous-preview".into())
            .unwrap();
        let id = || UnitId::parse("next").unwrap();
        assert!(stage(
            &mut registry,
            unit,
            id(),
            "art",
            prepared,
            &[],
            OutputTransport::Websocket
        )
        .is_err());
        assert!(registry.is_latest(unit, &old));
        assert_eq!(registry.preview(unit, &old), Some("previous-preview"));
        let inputs = [(
            "image",
            Input::InlinePng {
                base64: "iVBORw==",
                width: 1,
                height: 1,
            },
        )];
        assert!(matches!(
            stage(
                &mut registry,
                unit,
                id(),
                "",
                prepared,
                &inputs,
                OutputTransport::Websocket
            ),
            Err(Error::InvalidArtId)
        ));
        assert_eq!(registry.preview(unit, &old), Some("previous-preview"));
        let outgoing = stage(
            &mut registry,
            unit,
            id(),
            "art",
            prepared,
            &inputs,
            OutputTransport::Websocket,
        )
        .unwrap();
        assert_eq!(outgoing.request.generation, old.generation + 1);
        assert_eq!(outgoing.cancel, Some(cancel(unit, &old).unwrap()));
        assert!(outgoing
            .execute
            .starts_with("{\"method\":\"loom.hook.art.execute\""));
        assert!(registry.is_latest(unit, &outgoing.request));
        assert!(!registry.finish(unit, &old));
        assert_eq!(registry.preview_bytes(), 0);
    });
}
#[test]
fn actual_loom_envelope_uses_camel_case_and_resource_descriptors() {
    fixture(|unit, prepared| {
        let inputs = [(
            "image",
            Input::InlinePng {
                base64: "iVBORw==",
                width: 1,
                height: 1,
            },
        )];
        let wire = execute(
            unit,
            "art",
            &request(),
            prepared,
            &inputs,
            OutputTransport::Websocket,
        )
        .unwrap();
        assert_eq!(wire, "{\"method\":\"loom.hook.art.execute\",\"params\":{\"protocolVersion\":\"loom.hook.v1\",\"requestId\":\"request\",\"nodeId\":\"unit\",\"generation\":2,\"deviceId\":\"device:local\",\"artId\":\"art\",\"outputTransports\":[\"websocket\"],\"inputs\":{\"image\":{\"kind\":\"inline_resource\",\"mime\":\"image/png\",\"dataBase64\":\"iVBORw==\",\"width\":1,\"height\":1}},\"parameters\":{\"amount\":2},\"disabledParameters\":[]}}");
        println!("LOOM_FIXTURE:{wire}");
        assert!(!wire.contains("\"action\""));
        assert_eq!(cancel(unit, &request()).unwrap(), "{\"method\":\"loom.hook.art.cancel\",\"params\":{\"protocolVersion\":\"loom.hook.v1\",\"requestId\":\"request\",\"nodeId\":\"unit\",\"generation\":2,\"deviceId\":\"device:local\"}}");
    });
}
#[test]
fn shared_memory_transport_must_be_explicit_and_rgba_size_exact() {
    fixture(|unit, prepared| {
        let good = [(
            "image",
            Input::SharedRgba {
                handle: "otter-art-input-fixture",
                width: 2,
                height: 3,
                size: 24,
            },
        )];
        let wire = execute(
            unit,
            "art",
            &request(),
            prepared,
            &good,
            OutputTransport::SharedMemoryAndWebsocket,
        )
        .unwrap();
        assert!(wire.contains("\"outputTransports\":[\"shared_memory\",\"websocket\"]"));
        assert!(wire.contains("\"size\":24,\"format\":\"rgba8\""));
        let bad = [(
            "image",
            Input::SharedRgba {
                handle: "otter-art-input-fixture",
                width: 2,
                height: 3,
                size: 23,
            },
        )];
        assert_eq!(
            execute(
                unit,
                "art",
                &request(),
                prepared,
                &bad,
                OutputTransport::Websocket
            ),
            Err(Error::InvalidValue)
        );
    });
}
#[test]
fn resource_keys_dimensions_and_base64_are_validated_before_encoding() {
    fixture(|unit, prepared| {
        assert_eq!(
            execute(
                unit,
                "art",
                &request(),
                prepared,
                &[],
                OutputTransport::Websocket
            ),
            Err(Error::InvalidValue)
        );
        for (name, data, width, height) in [
            ("wrong", "AAAA", 1, 1),
            ("image", "AA=A", 1, 1),
            ("image", "A===", 1, 1),
            ("image", "AB==", 1, 1),
            ("image", "AAB=", 1, 1),
            ("image", "AAAA", 0, 1),
            ("image", "AAAA", 32768, 32768),
        ] {
            let input = [(
                name,
                Input::InlinePng {
                    base64: data,
                    width,
                    height,
                },
            )];
            assert_eq!(
                execute(
                    unit,
                    "art",
                    &request(),
                    prepared,
                    &input,
                    OutputTransport::Websocket
                ),
                Err(Error::InvalidValue)
            );
        }
    });
}
#[test]
fn batch_resident_budget_is_checked_even_for_small_descriptors() {
    fixture(|unit, prepared| {
        prepared.images.push(("mask", "mask-ref"));
        let inputs = [
            (
                "image",
                Input::SharedRgba {
                    handle: "otter-art-input-first",
                    width: 8192,
                    height: 8192,
                    size: 256 * 1024 * 1024,
                },
            ),
            (
                "mask",
                Input::SharedRgba {
                    handle: "otter-art-input-second",
                    width: 1,
                    height: 1,
                    size: 4,
                },
            ),
        ];
        assert_eq!(
            execute(
                unit,
                "art",
                &request(),
                prepared,
                &inputs,
                OutputTransport::Websocket
            ),
            Err(Error::MessageTooLarge)
        );
    });
}
