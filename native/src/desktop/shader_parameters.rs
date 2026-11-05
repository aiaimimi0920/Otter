//! 公共有效参数到单次 Shader 绑定的不可变快照；不解析路径/URL，也不保留图或 UI 借用。
use super::shader_payload::Error;
use crate::{graph_image::Output, graph_params::Params};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Bindings {
    pub(super) uniforms: Vec<(String, f32)>,
    pub(super) textures: BTreeMap<String, String>,
    pub(super) prepared: Vec<String>,
}
impl Bindings {
    pub fn capture(parameters: &Params<'_>, prepared: Vec<String>) -> Result<Self, Error> {
        let mut bindings = Self {
            prepared,
            ..Self::default()
        };
        let mut texture_bytes = 0usize;
        for (name, value) in parameters.entries() {
            // 旧 ShaderPreview 不将 reference 或内部 UI 参数直接上传到 GL。
            if name.starts_with("__")
                || name == "reference"
                || bindings.prepared.iter().any(|port| port == name)
            {
                continue;
            }
            let number = match value {
                Some(Output::Number(value)) => Some(value as f32),
                Some(Output::Boolean(value)) => Some(if value { 1.0 } else { 0.0 }),
                _ => None,
            };
            if let Some(number) = number {
                crate::shader_angle::name(name).map_err(|_| Error::Bindings)?;
                if !number.is_finite() || bindings.uniforms.len() >= 256 {
                    return Err(Error::Bindings);
                }
                bindings.uniforms.push((name.into(), number));
            } else if let Some(Output::Image(uri)) = value {
                // 非图像枚举/文本与旧 Image.onerror 一样不成为纹理；没有网络/文件回退。
                if !uri.starts_with("data:") {
                    continue;
                }
                if !uri.starts_with("data:image/png;base64,") {
                    return Err(Error::Unsupported);
                }
                crate::shader_angle::name(name).map_err(|_| Error::Bindings)?;
                if name == "input" || bindings.textures.len() >= 31 {
                    return Err(Error::Bindings);
                }
                texture_bytes = texture_bytes
                    .checked_add(uri.len())
                    .filter(|bytes| *bytes <= crate::graph_execution_wire::MAX_MESSAGE_BYTES)
                    .ok_or(Error::Budget)?;
                bindings.textures.insert(name.into(), uri.into());
            }
        }
        Ok(bindings)
    }
}
pub(crate) fn public_texture_name(name: &str, input: &str) -> bool {
    !name.starts_with("__")
        && !matches!(
            name,
            "reference" | "input" | "inputs" | "params" | "mode" | "output_mode"
        )
        && name != input
}
/// 声明图片端口或应用已捕获的本地 Unit 参数；这里不解析字符串或工作区。
pub(super) fn prepared_parameters(
    capability: &crate::loom_capability::Capability<'_>,
    parameters: &Params<'_>,
    input: &str,
    ports: &[&str],
    explicit: &[String],
) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    for parameter in capability.parameters().filter(|parameter| {
        !parameter.secret
            && public_texture_name(parameter.id, input)
            && matches!(parameter.widget.as_str(), "image_link" | "file")
            && ports.contains(&parameter.id)
            && parameters.entries().any(|(name, value)| {
                name == parameter.id && value != Some(Output::Image(crate::graph_params::DISABLED))
            })
    }) {
        crate::shader_angle::name(parameter.id).map_err(|_| Error::Bindings)?;
        if !names.iter().any(|name| name == parameter.id) {
            if names.len() >= 31 {
                return Err(Error::Bindings);
            }
            names.push(parameter.id.into());
        }
    }
    for name in explicit {
        if !public_texture_name(name, input)
            || !ports.contains(&name.as_str())
            || !parameters.entries().any(|(parameter, value)| {
                parameter == name
                    && matches!(value, Some(Output::Image(value)) if value != crate::graph_params::DISABLED)
            })
        {
            return Err(Error::Bindings);
        }
        crate::shader_angle::name(name).map_err(|_| Error::Bindings)?;
        if !names.contains(name) {
            if names.len() >= 31 {
                return Err(Error::Bindings);
            }
            names.push(name.clone());
        }
    }
    Ok(names)
}
#[cfg(test)]
#[path = "shader_parameter_tests.rs"]
mod tests;
