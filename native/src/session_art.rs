//! 将会话节点绑定到已匹配的能力；只构造候选，不执行、不修改活动文档。
use crate::{
    graph_execution_config::Config, loom_capability::Capability, loom_parameter_state::Parameters,
    session_parameters, session_window::Window,
};

pub struct Restored {
    pub parameters: Parameters,
    pub execution: Config,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Identity,
    ExecutionCapacity,
    ExecutionShape,
    Parameters(session_parameters::Error),
}

/// 调用方按主干规则精确查找 ID；防止错能力的 secret 策略被套用。
/// 禁用能力仍允许恢复状态；能否执行由执行入口另行检查。
pub fn restore(window: &Window<'_>, capability: &Capability<'_>) -> Result<Restored, Error> {
    if !window.is_art() || window.art_id.as_deref() != Some(capability.id) {
        return Err(Error::Identity);
    }
    let explicit: Option<serde_json::Value> = window
        .execution_config
        .map(|raw| {
            // 配置只含少量开关。限制解析分配，不将任意会话大字段复制进执行状态。
            if raw.get().len() > 64 * 1024 {
                return Err(Error::ExecutionCapacity);
            }
            serde_json::from_str(raw.get()).map_err(|_| Error::ExecutionShape)
        })
        .transpose()?;
    let execution = capability.execution_config(explicit.as_ref());
    let parameters =
        session_parameters::restore(capability, window.params).map_err(Error::Parameters)?;
    Ok(Restored {
        parameters,
        execution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capability() -> serde_json::Value {
        serde_json::json!({"id":"search","enabled":false,"execution":{"type":"mcp","toolName":"image_search"},
            "parameters":[{"id":"secret","secret":true},{"id":"amount","default":1}]})
    }
    #[test]
    fn restored_switches_and_parameters_use_the_same_capability() {
        let definition = capability();
        let capability = Capability::new(&definition).unwrap();
        let window: Window<'_> = serde_json::from_str(r#"{"id":"u","type":"art","artId":"search","x":0,"y":0,"w":1,"h":1,
            "executionConfig":{"triggerMode":{"paramDriven":false,"upstreamDriven":false},"propagation":{"notifyDownstream":false},"__expanded":false},
            "params":{"legacy":42,"secret":"hidden","amount":null}}"#).unwrap();
        let restored = restore(&window, &capability).unwrap();
        assert!(!restored.execution.param_driven);
        assert!(!restored.execution.upstream_driven);
        assert!(!restored.execution.notify_downstream);
        assert!(restored.execution.listen_upstream);
        assert_eq!(restored.execution.expanded, Some(false));
        assert_eq!(
            restored.parameters.get("amount"),
            Some(Some(&serde_json::Value::Null))
        );
        assert!(!restored.parameters.entries().any(|(id, _)| id == "secret"));
        assert!(restored.parameters.entries().any(|(id, _)| id == "legacy"));
    }
    #[test]
    fn missing_and_empty_config_keep_main_costly_search_defaults() {
        let definition = capability();
        let capability = Capability::new(&definition).unwrap();
        for (config, expected) in [
            ("null", false),
            ("{}", false),
            (r#"{"triggerMode":{}}"#, true),
        ] {
            let json = format!(
                r#"{{"id":"u","type":"art","artId":"search","x":0,"y":0,"w":1,"h":1,"executionConfig":{config}}}"#
            );
            let mut window: Window<'_> = serde_json::from_str(&json).unwrap();
            assert_eq!(
                restore(&window, &capability)
                    .unwrap()
                    .execution
                    .param_driven,
                expected
            );
            window.art_id = Some("different".into());
            assert!(matches!(
                restore(&window, &capability),
                Err(Error::Identity)
            ));
        }
    }
}
