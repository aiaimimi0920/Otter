//! 异步输入准备期间持有的不可变公共配置；不保留图借用、secret 或图片数据。
use super::*;
pub struct Configuration {
    pub(super) art_id: String,
    pub(super) image_ports: Vec<String>,
    pub(super) tail: String,
}
impl Configuration {
    /// 图层已完成公共字段筛选及连线转换；编码期间不回读 UI 状态。
    pub fn from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, false, false)
    }
    /// Shader 取码与本地绑定消费同份已解析公共参数；模式强制不受手动禁用影响。
    pub fn shader_from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, true, false)
    }
    /// ABI 由当前宿主安装决定，不接受脚本/连线/禁用参数覆盖。
    pub fn native_shader_from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, true, true)
    }
    fn resolved_mode(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
        shader: bool,
        native: bool,
    ) -> Result<Self, Error> {
        let image_ports = ports(art_id, images.iter().copied())?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        if shader {
            // Loom 合并 inputs/parameters；不能丢弃真实图片或以同名强制模式覆盖它。
            if image_ports.iter().any(|name| {
                matches!(name.as_str(), "mode" | "output_mode") || (native && name == "shader_abi")
            }) {
                return Err(Error::InvalidValue);
            }
            json.raw("\"output_mode\":\"shader\",\"mode\":\"shader\"")?;
            if native {
                json.raw(",\"shader_abi\":\"otter.hlsl.v1\"")?;
            }
            first = false;
        }
        for (name, value) in parameters.entries() {
            // Loom 将 inputs 与 parameters 合并为同一个参数对象；图片来源替代同名标量。
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            if (shader && matches!(name, "mode" | "output_mode"))
                || (native && name == "shader_abi")
            {
                continue;
            }
            let Some(value) = value else { continue };
            if matches!(
                value,
                Output::Other | Output::Image(crate::graph_params::DISABLED)
            ) {
                continue;
            }
            json.field(name, &mut first)?;
            json.value(value, 0)?;
        }
        finish(
            art_id,
            image_ports,
            json,
            parameters.entries().filter_map(|(name, value)| {
                (value == Some(Output::Image(crate::graph_params::DISABLED))
                    && !(shader && matches!(name, "mode" | "output_mode"))
                    && !(native && name == "shader_abi"))
                    .then_some(name)
            }),
        )
    }
    pub fn capture(art_id: &str, prepared: &Prepared<'_>) -> Result<Self, Error> {
        let image_ports = ports(art_id, prepared.images.iter().map(|(name, _)| *name))?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        for (name, value) in prepared.active_params() {
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            let Some(value) = value else {
                continue;
            };
            if matches!(value, Output::Other) {
                continue;
            }
            json.field(name, &mut first)?;
            json.value(value, 0)?;
        }
        finish(art_id, image_ports, json, prepared.disabled.iter().copied())
    }
    /// 从已筛选且有界的公共编辑状态生成独立快照，不绕过 Params 的 secret/内部字段边界。
    pub fn from_parameters(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::parameters_mode(parameters, images, false)
    }
    /// 仍只从公共 Parameters 读取；强制预取 Shader 模式，图片沿正式 inputs 描述符传输。
    pub fn shader_from_parameters(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::parameters_mode(parameters, images, true)
    }
    fn parameters_mode(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
        shader: bool,
    ) -> Result<Self, Error> {
        let image_ports = ports(parameters.art_id(), images.iter().copied())?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        if shader {
            // 与 resolved 快照保持同一准入规则；普通 Art 仍允许这两个图片端口名。
            if image_ports
                .iter()
                .any(|name| matches!(name.as_str(), "mode" | "output_mode"))
            {
                return Err(Error::InvalidValue);
            }
            json.raw("\"output_mode\":\"shader\",\"mode\":\"shader\"")?;
            first = false;
        }
        for (name, value) in parameters.entries() {
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            if shader && matches!(name, "output_mode" | "mode") {
                continue;
            }
            let Some(value) = value else {
                continue;
            };
            if disabled(value) {
                continue;
            }
            json.field(name, &mut first)?;
            let encoded = serde_json::to_string(value).map_err(|_| Error::InvalidValue)?;
            json.raw(&encoded)?;
        }
        finish(
            parameters.art_id(),
            image_ports,
            json,
            parameters.entries().filter_map(|(name, value)| {
                (value.is_some_and(disabled) && !(shader && matches!(name, "output_mode" | "mode")))
                    .then_some(name)
            }),
        )
    }
}
fn disabled(value: &serde_json::Value) -> bool {
    value.as_str() == Some(crate::graph_params::DISABLED)
}
#[cfg(test)]
#[path = "loom_art_config_tests.rs"]
mod tests;
fn ports<'a>(art_id: &str, names: impl Iterator<Item = &'a str>) -> Result<Vec<String>, Error> {
    if art_id.is_empty() || art_id.len() > 256 || art_id.chars().any(char::is_control) {
        return Err(Error::InvalidArtId);
    }
    let mut image_ports = Vec::new();
    for name in names {
        if image_ports.len() >= 32
            || name.is_empty()
            || name.len() > 256
            || name.chars().any(char::is_control)
            || image_ports.iter().any(|previous| previous == name)
        {
            return Err(Error::InvalidValue);
        }
        image_ports.try_reserve(1).map_err(|_| Error::Allocation)?;
        image_ports.push(name.to_owned());
    }
    Ok(image_ports)
}
fn finish<'a>(
    art_id: &str,
    image_ports: Vec<String>,
    mut json: Json,
    disabled: impl Iterator<Item = &'a str>,
) -> Result<Configuration, Error> {
    json.raw("},\"disabledParameters\":[")?;
    for (index, name) in disabled.enumerate() {
        if index != 0 {
            json.raw(",")?;
        }
        json.string(name)?;
    }
    json.raw("]")?;
    Ok(Configuration {
        art_id: art_id.into(),
        image_ports,
        tail: json.0,
    })
}
