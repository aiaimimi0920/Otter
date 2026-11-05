//! 已绑定 Art 的公共参数/执行配置写回；旧 params 不参与合并，避免恢复已经过滤的 secret。
use crate::{graph_execution_config::Config, loom_parameter_state::Parameters};
use serde_json::{Map, Value};

pub fn merge(
    mut window: Map<String, Value>,
    parameters: &Parameters,
    execution: Config,
) -> Map<String, Value> {
    let same_art = window.get("artId").and_then(Value::as_str) == Some(parameters.art_id());
    let mut config = if same_art {
        match window.remove("executionConfig") {
            Some(Value::Object(config)) => config,
            _ => Map::new(),
        }
    } else {
        // 更换能力时不把旧能力的未知执行配置带给新能力。
        window.remove("executionConfig");
        Map::new()
    };
    let mut trigger = object(config.remove("triggerMode"));
    trigger.insert("upstreamDriven".into(), execution.upstream_driven.into());
    trigger.insert("paramDriven".into(), execution.param_driven.into());
    let mut propagation = object(config.remove("propagation"));
    propagation.insert("listenUpstream".into(), execution.listen_upstream.into());
    propagation.insert(
        "notifyDownstream".into(),
        execution.notify_downstream.into(),
    );
    config.insert("triggerMode".into(), trigger.into());
    config.insert("propagation".into(), propagation.into());
    match execution.expanded {
        Some(value) => {
            config.insert("__expanded".into(), value.into());
        }
        None => {
            config.remove("__expanded");
        }
    }
    let params: Map<String, Value> = parameters
        .entries()
        .filter_map(|(id, value)| value.map(|value| (id.to_owned(), value.clone())))
        .collect();
    window.insert("type".into(), "art".into());
    window.insert("artId".into(), parameters.art_id().into());
    window.insert("params".into(), params.into());
    window.insert("executionConfig".into(), config.into());
    window
}
fn object(value: Option<Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(object)) => object,
        _ => Map::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{loom_capability::Capability, session_art, session_window::Window};
    use serde_json::json;
    fn definition() -> Value {
        json!({"id":"art","execution":{"type":"mcp","toolName":"image_search"},
        "parameters":[{"id":"unset","default":5},{"id":"empty"},{"id":"off"},{"id":"secret","secret":true}]})
    }
    fn window() -> Map<String, Value> {
        json!({"id":"u","x":0,"y":0,"w":20,"h":20,"type":"art","artId":"art","extension":{"keep":true},
            "params":{"secret":"must-not-return","removed":"old"},
            "executionConfig":{"extension":7,"__expanded":true,"triggerMode":{"extension":8},"propagation":{"extension":9}}})
            .as_object().unwrap().clone()
    }
    #[test]
    fn public_values_replace_old_params_and_preserve_missing_null_disabled_distinction() {
        let definition = definition();
        let capability = Capability::new(&definition).unwrap();
        let values = json!({"empty":null,"off":"__DISABLED__","secret":"hidden","legacy":42,"__exec_mode":"ignore"});
        let parameters =
            Parameters::from_persisted(&capability, values.as_object().unwrap().clone()).unwrap();
        let execution = Config {
            upstream_driven: false,
            param_driven: false,
            listen_upstream: true,
            notify_downstream: false,
            expanded: None,
        };
        let saved = merge(window(), &parameters, execution);
        let params = saved["params"].as_object().unwrap();
        assert!(!params.contains_key("unset"));
        assert!(!params.contains_key("secret"));
        assert!(!params.contains_key("removed"));
        assert!(!params.contains_key("__exec_mode"));
        assert_eq!(params["empty"], Value::Null);
        assert_eq!(params["off"], "__DISABLED__");
        assert_eq!(params["legacy"], 42);
        assert_eq!(saved["extension"], json!({"keep":true}));
        assert_eq!(saved["executionConfig"]["extension"], 7);
        assert_eq!(saved["executionConfig"]["triggerMode"]["extension"], 8);
        assert_eq!(saved["executionConfig"]["propagation"]["extension"], 9);
        assert!(saved["executionConfig"].get("__expanded").is_none());
        let text = Value::Object(saved).to_string();
        let window: Window<'_> = serde_json::from_str(&text).unwrap();
        let restored = session_art::restore(&window, &capability).unwrap();
        assert_eq!(restored.execution, execution);
        assert_eq!(restored.parameters.get("unset"), Some(None));
        assert_eq!(restored.parameters.get("empty"), Some(Some(&Value::Null)));
        assert_eq!(
            restored.parameters.entries().collect::<Vec<_>>(),
            parameters.entries().collect::<Vec<_>>()
        );
    }
    #[test]
    fn changed_art_does_not_inherit_old_execution_extensions_or_parameters() {
        let definition = json!({"id":"other","parameters":[{"id":"fresh","default":1}]});
        let parameters = Parameters::new(&Capability::new(&definition).unwrap()).unwrap();
        let saved = merge(
            window(),
            &parameters,
            Config {
                expanded: Some(false),
                ..Config::default()
            },
        );
        assert_eq!(saved["artId"], "other");
        assert_eq!(saved["params"], json!({"fresh":1}));
        assert!(saved["executionConfig"].get("extension").is_none());
        assert!(saved["executionConfig"]["triggerMode"]
            .get("extension")
            .is_none());
        assert_eq!(saved["executionConfig"]["__expanded"], false);
    }
}
