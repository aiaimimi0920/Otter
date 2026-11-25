//! 执行配置与正式请求准入；不把本地 shader/surface 预览混入后端执行。
use crate::graph_image::Output;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub upstream_driven: bool,
    pub param_driven: bool,
    pub listen_upstream: bool,
    pub notify_downstream: bool,
    pub expanded: Option<bool>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            upstream_driven: true,
            param_driven: true,
            listen_upstream: true,
            notify_downstream: true,
            expanded: None,
        }
    }
}
fn field<'a>(value: Option<Output<'a>>, key: &str) -> Option<Output<'a>> {
    if let Some(Output::Json(serde_json::Value::Object(fields))) = value {
        return fields.get(key).map(Output::from_json);
    }
    let Output::Object(fields) = value? else {
        return None;
    };
    fields
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
}
fn boolean(value: Option<Output<'_>>, fallback: bool) -> bool {
    if let Some(Output::Boolean(value)) = value {
        value
    } else {
        fallback
    }
}
impl Config {
    /// 保持主干区分：显式空 triggerMode/propagation 对象也算配置，字符串 true 不算布尔。
    pub fn derive(explicit: Option<Output<'_>>, execution_type: &str, tool_name: &str) -> Self {
        let trigger = field(explicit, "triggerMode");
        let propagation = field(explicit, "propagation");
        let object = |value| {
            matches!(
                value,
                Some(Output::Object(_) | Output::Json(serde_json::Value::Object(_)))
            )
        };
        let meaningful = object(trigger) || object(propagation);
        let contains = |needle: &[u8]| {
            tool_name
                .as_bytes()
                .windows(needle.len())
                .any(|part| part.eq_ignore_ascii_case(needle))
        };
        let costly = execution_type == "mcp" && contains(b"image") && contains(b"search");
        Self {
            upstream_driven: boolean(field(trigger, "upstreamDriven"), true),
            param_driven: boolean(field(trigger, "paramDriven"), meaningful || !costly),
            listen_upstream: boolean(field(propagation, "listenUpstream"), true),
            notify_downstream: boolean(field(propagation, "notifyDownstream"), true),
            expanded: match field(explicit, "__expanded") {
                Some(Output::Boolean(value)) => Some(value),
                _ => None,
            },
        }
    }
    pub fn accepts_upstream_notification(&self) -> bool {
        self.listen_upstream && self.upstream_driven
    }
    /// 上游 listen/来源 notify 在传播入口检查；此处对应 handleParamChange 的正式执行门槛。
    pub fn gate(&self, trigger: Trigger, runtime: Runtime) -> Result<(), Skip> {
        if runtime.surface && !runtime.formal_after_preview {
            return Err(Skip::SurfaceOnly);
        }
        match trigger {
            Trigger::Parameter { .. } if !self.param_driven => return Err(Skip::ParameterDisabled),
            Trigger::Upstream if !self.upstream_driven => return Err(Skip::UpstreamDisabled),
            _ => {}
        }
        if runtime.shader && !runtime.formal_after_preview {
            return Err(Skip::ShaderOnly);
        }
        if trigger
            == (Trigger::Parameter {
                final_change: false,
            })
        {
            return Err(Skip::NonFinalParameter);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trigger {
    Parameter { final_change: bool },
    Upstream,
    Manual,
}
#[derive(Clone, Copy, Default)]
pub struct Runtime {
    pub surface: bool,
    pub shader: bool,
    pub formal_after_preview: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Skip {
    SurfaceOnly,
    ShaderOnly,
    ParameterDisabled,
    UpstreamDisabled,
    NonFinalParameter,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expensive_search_defaults_are_overridden_only_by_meaningful_objects() {
        assert!(!Config::derive(None, "mcp", "Search_Images").param_driven);
        assert!(Config::derive(None, "MCP", "Search_Images").param_driven);
        assert!(Config::derive(None, "mcp", "search_text").param_driven);
        let empty = [("triggerMode", Output::Object(&[]))];
        assert!(Config::derive(Some(Output::Object(&empty)), "mcp", "image_search").param_driven);
        let invalid = [
            ("triggerMode", Output::Array(&[])),
            ("__expanded", Output::Boolean(true)),
        ];
        let config = Config::derive(Some(Output::Object(&invalid)), "mcp", "image_search");
        assert!(!config.param_driven);
        assert_eq!(config.expanded, Some(true));
    }
    #[test]
    fn flags_accept_only_boolean_and_manual_bypasses_auto_trigger_switches() {
        let flags = [
            ("upstreamDriven", Output::Boolean(false)),
            ("paramDriven", Output::Image("false")),
        ];
        let source = [("triggerMode", Output::Object(&flags))];
        let mut config = Config::derive(Some(Output::Object(&source)), "mcp", "image_search");
        assert!(config.param_driven);
        assert!(!config.upstream_driven);
        assert_eq!(
            config.gate(Trigger::Upstream, Runtime::default()),
            Err(Skip::UpstreamDisabled)
        );
        config.param_driven = false;
        assert_eq!(config.gate(Trigger::Manual, Runtime::default()), Ok(()));
        assert!(!config.accepts_upstream_notification());
    }
    #[test]
    fn preview_only_and_nonfinal_changes_never_dispatch_formal_work() {
        let config = Config::default();
        let shader = Runtime {
            shader: true,
            ..Runtime::default()
        };
        let surface = Runtime {
            surface: true,
            ..Runtime::default()
        };
        assert_eq!(config.gate(Trigger::Manual, shader), Err(Skip::ShaderOnly));
        assert_eq!(
            config.gate(Trigger::Manual, surface),
            Err(Skip::SurfaceOnly)
        );
        let hybrid = Runtime {
            formal_after_preview: true,
            ..shader
        };
        assert_eq!(config.gate(Trigger::Manual, hybrid), Ok(()));
        assert_eq!(
            config.gate(
                Trigger::Parameter {
                    final_change: false
                },
                hybrid
            ),
            Err(Skip::NonFinalParameter)
        );
        assert_eq!(
            config.gate(Trigger::Parameter { final_change: true }, hybrid),
            Ok(())
        );
    }
}
