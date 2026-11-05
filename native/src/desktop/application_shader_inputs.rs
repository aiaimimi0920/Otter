//! 公共参数中的显式本地 Unit 引用；复用唯一 Graph/Source，不授予路径、网络或预览读取权。
use crate::{
    graph_image::{Graph, Output},
    graph_params::Params,
    loom_capability::Capability,
    workspace::Workspace,
    workspace_id::UnitId,
};

pub(super) fn references<'a>(
    workspace: &'a Workspace,
    parameters: Option<&Params<'_>>,
    input: &str,
) -> Result<Vec<(String, &'a UnitId)>, &'static str> {
    let mut references = Vec::new();
    let Some(parameters) = parameters else {
        return Ok(references);
    };
    for (name, value) in parameters.entries() {
        if !crate::desktop::shader_parameters::public_texture_name(name, input) {
            continue;
        }
        let Some(Output::Image(value)) = value else {
            continue;
        };
        if value == crate::graph_params::DISABLED {
            continue;
        }
        // 必须逐字匹配当前 workspace 的身份；未知字符串/URL/路径不是读取请求。
        let Some(unit) = workspace.units().iter().find(|unit| unit.as_str() == value) else {
            continue;
        };
        crate::shader_angle::name(name).map_err(|_| "Shader 图片参数名称无效")?;
        if references.len() >= 31 {
            return Err("Shader 图片参数数量超过预算");
        }
        references
            .try_reserve(1)
            .map_err(|_| "无法分配 Shader 参数引用")?;
        references.push((name.into(), unit));
    }
    Ok(references)
}
#[cfg(test)]
#[path = "application_shader_input_tests.rs"]
mod tests;

pub(super) fn resolve<'a>(
    shader: Option<&Capability<'_>>,
    graph: &Graph<'a>,
    references: &'a [(String, &UnitId)],
    images: &mut Vec<(&'a str, &'a str)>,
) -> Result<Prepared<'a>, &'static str> {
    let mut prepared = Prepared::default();
    let ports: Vec<_> = images.iter().map(|(name, _)| *name).collect();
    let Some(input) = shader.and_then(|capability| capability.available_shader_input(&ports))
    else {
        return Ok(prepared);
    };
    for (name, unit) in references {
        if !crate::desktop::shader_parameters::public_texture_name(name, input) {
            continue;
        }
        if !images.iter().any(|(port, _)| *port == name) {
            // Art 只能取正式 output；Sticker 继续由 Graph 处理 relay/编辑/Live 的来源。
            // 不调用 image()/canvas_image() 取得 Art 自身 display 或恢复预览。
            let Some(Output::Image(key)) = graph.output(unit, "output").map(Output::direct) else {
                return Err("Shader 参数引用暂无可用正式图片");
            };
            images
                .try_reserve(1)
                .map_err(|_| "无法分配 Shader 参数图片")?;
            images.push((name.as_str(), key));
            prepared
                .units
                .try_reserve(1)
                .map_err(|_| "无法冻结 Shader 参数来源")?;
            prepared.units.push(*unit);
        }
        prepared
            .parameters
            .try_reserve(1)
            .map_err(|_| "无法冻结 Shader 参数图片")?;
        prepared.parameters.push(name.clone());
    }
    Ok(prepared)
}
#[derive(Default)]
pub(super) struct Prepared<'a> {
    pub parameters: Vec<String>,
    pub units: Vec<&'a UnitId>,
}
