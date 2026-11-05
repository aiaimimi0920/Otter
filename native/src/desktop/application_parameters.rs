//! 同一 UI 调度段借用正式输出解析参数，编码后仅保留配置字符串和版本凭证。
use super::Application;
use crate::desktop::{
    configuration_stamp::Stamp, document_window::DocumentWindow, loom_input_stamp::SourceStamp,
    loom_output_batch::Output as ResultOutput,
};
use crate::{
    graph_image::{Graph, Kind, Node, Output},
    graph_param_coercion::Widget,
    graph_params::Param,
    loom_art_wire::Configuration,
    loom_capability::Capability,
    loom_parameter_state::Parameters,
    workspace::Workspace,
    workspace_id::UnitId,
};
type Captured<T = Configuration> = (T, Vec<(UnitId, SourceStamp)>);
type ExecutionPlan = (
    Configuration,
    Option<crate::desktop::shader_prepare::Runtime>,
    Vec<crate::desktop::loom_input_job::Source>,
);
impl Application {
    pub(super) fn execution_parameters(
        documents: &[DocumentWindow],
        workspace: &Workspace,
        target: &UnitId,
        capability: &Capability<'_>,
        manual: &Parameters,
        shader: Option<&crate::desktop::shader_prepare::Installation>,
        inputs: impl FnOnce(
            &crate::graph_params::Params<'_>,
        ) -> Result<Option<super::image_inputs::ImageInputs>, &'static str>,
    ) -> Result<Captured<Option<ExecutionPlan>>, &'static str> {
        Self::with_parameters(
            documents,
            workspace,
            target,
            capability,
            manual,
            |resolved| {
                // 来源、取码配置与本地纹理使用同一次公共参数解析，不再于准备之后重读参数。
                let Some(inputs) = inputs(resolved)? else {
                    return Ok(None);
                };
                let images: Vec<_> = inputs.ports.iter().map(String::as_str).collect();
                let configuration =
                    if shader.is_some_and(|installation| installation.bundle.is_none()) {
                        Configuration::native_shader_from_resolved(capability.id, resolved, &images)
                    } else if shader.is_some() {
                        Configuration::shader_from_resolved(capability.id, resolved, &images)
                    } else {
                        Configuration::from_resolved(capability.id, resolved, &images)
                    }
                    .map_err(|_| "无法捕获执行参数")?;
                let shader = shader
                    .map(|installation| {
                        installation.capture(
                            capability,
                            &images,
                            resolved,
                            &inputs.shader_parameters,
                        )
                    })
                    .transpose()
                    .map_err(|_| "无法准备本地 Shader 参数或主图像输入")?;
                Ok(Some((configuration, shader, inputs.sources)))
            },
        )
    }
    pub(super) fn with_parameters<T>(
        documents: &[DocumentWindow],
        workspace: &Workspace,
        target: &UnitId,
        capability: &Capability<'_>,
        manual: &Parameters,
        consume: impl FnOnce(&crate::graph_params::Params<'_>) -> Result<T, &'static str>,
    ) -> Result<Captured<T>, &'static str> {
        let upstream = workspace
            .upstream_units(target)
            .map_err(|_| "无法解析参数依赖")?;
        let documents: Vec<_> = documents
            .iter()
            .filter(|doc| upstream.contains(&&doc.state.unit_id))
            .collect();
        let results: Vec<_> = documents
            .iter()
            .map(|doc| doc.state.execution_results.borrow())
            .collect();
        let values: Vec<Vec<_>> = results
            .iter()
            .map(|result| {
                result
                    .ports()
                    .iter()
                    .map(|port| {
                        (
                            port.name.as_str(),
                            match &port.value {
                                ResultOutput::Value(value) => Output::from_json(value),
                                // 图像内存不是路径字符串，不把像素或显示预览伪装为标量引用。
                                ResultOutput::Image(_) | ResultOutput::PngImage(_) => Output::Null,
                            },
                        )
                    })
                    .collect()
            })
            .collect();
        let nodes: Vec<_> = documents
            .iter()
            .enumerate()
            .map(|(index, doc)| Node {
                id: &doc.state.unit_id,
                kind: if doc.state.is_art() {
                    Kind::Art
                } else {
                    Kind::Sticker
                },
                inputs: &[],
                outputs: &[],
                capability_inputs: Some(&[]),
                capability_outputs: &[],
                values: &values[index],
                source: None,
                preview: None,
                manual_image: None,
                input_disabled: false,
                accept_upstream: true,
                locally_edited: false,
                pending: false,
            })
            .collect();
        let definitions: Vec<_> = capability.parameters().collect();
        let declared: Vec<_> = definitions
            .iter()
            .map(|param| Param {
                id: param.id,
                default: param.default.map(Output::from_json),
                secret: param.secret,
                widget: Widget {
                    name: &param.widget,
                    min: param.min,
                    max: param.max,
                },
            })
            .collect();
        let manual: Vec<_> = manual
            .entries()
            .filter_map(|(name, value)| value.map(|value| (name, Output::from_json(value))))
            .collect();
        let graph = Graph::new(&nodes, workspace.links()).map_err(|_| "参数图超过预算")?;
        let resolved = graph
            .effective_params(target, &declared, &manual)
            .map_err(|_| "无法解析连线参数")?;
        let configuration = consume(&resolved)?;
        let mut dependencies = Vec::new();
        dependencies
            .try_reserve(documents.len() * 2)
            .map_err(|_| "无法分配参数依赖")?;
        for (doc, result) in documents.iter().zip(&results) {
            let canvas = doc.state.canvas.borrow();
            if canvas.editor.draft().is_some() || doc.state.pending_adoption.borrow().is_some() {
                return Err("参数依赖存在未完成编辑");
            }
            dependencies.push((
                doc.state.unit_id.clone(),
                SourceStamp::Node {
                    document: canvas.editor.document_stamp(),
                    configuration: Stamp::capture(&doc.state),
                },
            ));
            dependencies.push((
                doc.state.unit_id.clone(),
                SourceStamp::Output(result.output_stamp()),
            ));
        }
        Ok((configuration, dependencies))
    }
}
