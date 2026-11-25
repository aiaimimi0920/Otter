//! 能力执行归属对齐 artCapabilities/nodeExecutionConfig，不按框架 ID 猜测预览类型。
use super::Capability;
use crate::graph_execution_config::{Config, Runtime};
use crate::graph_image::Output;
impl Capability<'_> {
    /// 对齐旧 unitSurfaceController 的参考图推导；它是预览上下文，不授予图片读取权。
    pub fn shader_reference_port(&self) -> Option<&str> {
        if let Some(name) = super::text(
            &self.raw["metadata"]["capabilities"],
            "shaderReferenceInput",
        ) {
            return Some(name);
        }
        self.inputs()
            .filter(shader_image_port)
            .find(|port| port.name.eq_ignore_ascii_case("reference"))
            .or_else(|| {
                self.inputs()
                    .filter(shader_image_port)
                    .find(|port| Some(port.name) != self.shader_input_port())
            })
            .map(|port| port.name)
    }
    pub fn shader_requires_reference(&self) -> bool {
        self.shader_reference_port().is_some()
            || self
                .parameters()
                .any(|parameter| parameter.widget == "image_link" || parameter.id == "reference")
            || self.inputs().any(|port| port.name == "reference")
    }
    /// 对齐 artCapabilities.shaderInputPortName；只选端口，不读取图片或把 reference 自动绑定为纹理。
    pub fn shader_input_port(&self) -> Option<&str> {
        if let Some(configured) = super::text(&self.raw["metadata"]["capabilities"], "shaderInput")
        {
            return Some(configured);
        }
        self.inputs()
            .filter(shader_image_port)
            .find(|port| port.is_primary_image())
            .or_else(|| self.inputs().find(shader_image_port))
            .map(|port| port.name)
    }
    /// 只在已准备端口中按旧 controller 顺序选取；不把其它辅助图片当主图。
    pub fn available_shader_input<'a>(&self, ports: &[&'a str]) -> Option<&'a str> {
        self.shader_input_port()
            .and_then(|name| ports.iter().copied().find(|port| *port == name))
            .or_else(|| {
                ["input", "input_image", "image"]
                    .into_iter()
                    .find_map(|name| ports.iter().copied().find(|port| *port == name))
            })
    }
    pub fn execution_config(&self, explicit: Option<&serde_json::Value>) -> Config {
        Config::derive(
            explicit.map(Output::from_json),
            self.raw["execution"]["type"].as_str().unwrap_or(""),
            self.raw["execution"]["toolName"].as_str().unwrap_or(""),
        )
    }
    pub fn runtime(&self) -> Runtime {
        let metadata = &self.raw["metadata"]["capabilities"];
        let surface = &metadata["surface"];
        let variants = surface["variants"].as_array();
        Runtime {
            shader: metadata["preview"].as_str() == Some("shader")
                || metadata["shader"].as_bool() == Some(true),
            surface: surface["fallbackScene"].is_string()
                || variants.is_some_and(|variants| {
                    variants.iter().any(|variant| {
                        matches!(
                            variant["runtime"].as_str(),
                            Some("declarative" | "javascript")
                        )
                    })
                }),
            formal_after_preview: metadata["requiresFormalExecution"].as_bool() == Some(true)
                || self.raw["execution"]["type"].as_str() == Some("workflow"),
        }
    }
}
// Shader 端口启发式与旧预览一致；不同于通用图端口按名称补充 image-like 的规则。
fn shader_image_port(port: &super::Port<'_>) -> bool {
    [Some(port.kind), port.execution_type]
        .into_iter()
        .flatten()
        .any(|value| value.to_ascii_lowercase().contains("image"))
        || port.widget == Some("image_link")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_execution_config::{Skip, Trigger};
    use serde_json::json;
    #[test]
    fn shader_reference_keeps_image_filter_case_and_legacy_context_heuristic() {
        let mut raw = json!({"id":"shader","inputs":[
            {"name":"input","type":"image"},
            {"name":"reference_image","type":"text"},
            {"name":"REFerence","widget":"image_link"}]});
        assert_eq!(
            Capability::new(&raw).unwrap().shader_reference_port(),
            Some("REFerence")
        );
        assert!(Capability::new(&raw).unwrap().shader_requires_reference());
        raw["inputs"][2] = json!({"name":"aux","execution_type":"IMAGE"});
        assert_eq!(
            Capability::new(&raw).unwrap().shader_reference_port(),
            Some("aux")
        );
        raw["metadata"] = json!({"capabilities":{"shaderReferenceInput":" custom "}});
        assert_eq!(
            Capability::new(&raw).unwrap().shader_reference_port(),
            Some("custom")
        );
        raw["metadata"] = json!({});
        raw["inputs"] =
            json!([{"name":"input","type":"image"}, {"name":"reference_image","type":"text"}]);
        assert!(!Capability::new(&raw).unwrap().shader_requires_reference());
        for definition in [
            json!({"parameters":[{"id":"lut","widget":"image_link"}]}),
            json!({"parameters":[{"id":"reference"}]}),
            json!({"inputs":[{"name":"reference","type":"text"}]}),
        ] {
            let mut value = json!({"id":"shader"});
            value
                .as_object_mut()
                .unwrap()
                .extend(definition.as_object().unwrap().clone());
            assert!(Capability::new(&value).unwrap().shader_requires_reference());
        }
    }
    #[test]
    fn shader_primary_uses_explicit_metadata_then_named_image_then_first_image() {
        let mut raw = json!({"id":"test","inputs":[{"name":"reference","type":"image"},{"name":"INPUT_IMAGE","widget":"image_link"},{"name":"not_image","type":"text"}]});
        assert_eq!(
            Capability::new(&raw).unwrap().shader_input_port(),
            Some("INPUT_IMAGE")
        );
        raw["metadata"] = json!({"capabilities":{"shaderInput":" reference "}});
        assert_eq!(
            Capability::new(&raw).unwrap().shader_input_port(),
            Some("reference")
        );
        raw["metadata"] = json!({});
        raw["inputs"][1]["widget"] = json!("text");
        assert_eq!(
            Capability::new(&raw).unwrap().shader_input_port(),
            Some("reference")
        );
        raw["inputs"] = json!([]);
        assert_eq!(Capability::new(&raw).unwrap().shader_input_port(), None);
    }
    #[test]
    fn shader_available_input_keeps_declared_and_standard_fallback_order() {
        let raw = json!({"id":"shader","metadata":{"capabilities":{"shaderInput":"source"}}});
        let cap = Capability::new(&raw).unwrap();
        assert_eq!(
            cap.available_shader_input(&["reference", "image"]),
            Some("image")
        );
        assert_eq!(
            cap.available_shader_input(&["image", "input_image", "input"]),
            Some("input")
        );
        assert_eq!(
            cap.available_shader_input(&["input", "source"]),
            Some("source")
        );
        assert_eq!(cap.available_shader_input(&["reference"]), None);
    }
    #[test]
    fn metadata_controls_preview_and_workflow_still_requires_formal_execution() {
        for capabilities in [
            json!({"preview":"shader"}),
            json!({"shader":true}),
            json!({"surface":{"variants":[{"runtime":"declarative"}]}}),
            json!({"surface":{"variants":[{"runtime":"javascript"}]}}),
            json!({"surface":{"fallbackScene":""}}),
        ] {
            let mut raw = json!({"id":"test","metadata":{"capabilities":capabilities}});
            let cap = Capability::new(&raw).unwrap();
            assert!(cap
                .execution_config(None)
                .gate(Trigger::Manual, cap.runtime())
                .is_err());
            raw["execution"] = json!({"type":"workflow"});
            let cap = Capability::new(&raw).unwrap();
            assert_eq!(
                cap.execution_config(None)
                    .gate(Trigger::Manual, cap.runtime()),
                Ok(())
            );
        }
        let raw = json!({"id":"test","framework":"shader","metadata":{"capabilities":{
            "shader":"true","preview":"Shader","surface":{"variants":[{"runtime":"unknown"}]}
        }}});
        let cap = Capability::new(&raw).unwrap();
        assert!(!cap.runtime().shader && !cap.runtime().surface);
    }
    #[test]
    fn explicit_formal_flag_and_expensive_search_defaults_match_main() {
        let raw = json!({"id":"test","execution":{"type":"mcp","toolName":"SEARCH_IMAGES"},
            "metadata":{"capabilities":{"shader":true,"requiresFormalExecution":true}}});
        let cap = Capability::new(&raw).unwrap();
        assert_eq!(
            cap.execution_config(None)
                .gate(Trigger::Parameter { final_change: true }, cap.runtime()),
            Err(Skip::ParameterDisabled)
        );
        assert_eq!(
            cap.execution_config(None)
                .gate(Trigger::Manual, cap.runtime()),
            Ok(())
        );
        assert!(
            cap.execution_config(Some(&json!({"triggerMode":{}})))
                .param_driven
        );
        let config = cap.execution_config(Some(&json!({"triggerMode":{"upstreamDriven":false},
            "propagation":{"listenUpstream":false},"__expanded":true})));
        assert!(!config.upstream_driven && !config.listen_upstream);
        assert_eq!(config.expanded, Some(true));
    }
}
