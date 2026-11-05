//! 工作区到图解析器的短生命周期视图；仅解析本地已拥有的图像，不访问路径/URL。
use super::Application;
use crate::{
    desktop::loom_input_job::Source,
    graph_image::{Graph, Kind, Node, Output, Port},
    loom_capability::Capability,
    workspace_id::UnitId,
};
#[path = "application_shader_inputs.rs"]
mod shader_inputs;
struct Snapshot {
    id: UnitId,
    kind: Kind,
    ports: Vec<(String, String)>,
    values: Vec<(String, Option<String>)>,
    source: Option<String>,
    edited: bool,
    accept_upstream: bool,
    sticker: crate::desktop::sticker_input::Config,
}
struct Origin {
    key: String,
    document: usize,
    port: Option<String>,
}
pub(in crate::desktop) struct ImageInputs {
    pub ports: Vec<String>,
    pub sources: Vec<Source>,
    pub shader_parameters: Vec<String>,
}
impl Application {
    pub(in crate::desktop) fn image_inputs(
        documents: &[crate::desktop::document_window::DocumentWindow],
        sessions: &[crate::desktop::composition::live::session::Session],
        workspace: &crate::workspace::Workspace,
        target: &UnitId,
        manual_trigger: bool,
        shader: Option<&Capability<'_>>,
        resolved: Option<&crate::graph_params::Params<'_>>,
    ) -> Result<Option<ImageInputs>, &'static str> {
        let mut dependencies = workspace
            .upstream_units(target)
            .map_err(|_| "无法解析节点依赖")?;
        let target_dependencies = dependencies.clone();
        let references = if let Some(shader) = shader {
            shader_inputs::references(
                workspace,
                resolved,
                shader.shader_input_port().unwrap_or("input"),
            )?
        } else {
            Vec::new()
        };
        // 显式 Unit 参数可指向未连线节点；仍用同一图视图及 workspace 的有界依赖遍历。
        for (_, unit) in &references {
            for dependency in workspace
                .upstream_units(unit)
                .map_err(|_| "无法解析 Shader 参数依赖")?
            {
                if !dependencies.contains(&dependency) {
                    dependencies
                        .try_reserve(1)
                        .map_err(|_| "无法分配图片依赖")?;
                    dependencies.push(dependency);
                }
            }
        }
        let mut snapshots = Vec::new();
        let mut origins = Vec::new();
        for (index, document) in documents.iter().enumerate() {
            if !dependencies.contains(&&document.state.unit_id) {
                continue;
            }
            let state = &document.state;
            if target_dependencies.contains(&&state.unit_id)
                && state.pending_adoption.borrow().is_some()
            {
                return Err("图片依赖存在未完成接管");
            }
            let canvas = state.canvas.borrow();
            let selected = state.selected_art.borrow();
            let kind = if state.is_art() {
                Kind::Art
            } else {
                Kind::Sticker
            };
            let ports = if let Some(selected) = selected.as_ref() {
                selected
                    .catalog
                    .handshake
                    .arts()
                    .iter()
                    .filter_map(Capability::new)
                    .find(|cap| cap.id == selected.parameters.art_id())
                    .map(|cap| {
                        cap.inputs()
                            .map(|port| {
                                (
                                    port.name.into(),
                                    if port.is_image_like() {
                                        "image".into()
                                    } else {
                                        port.kind.into()
                                    },
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else if let Some(local) = &state.local_art_id {
                local.inputs()
            } else {
                vec![("image".into(), "image".into())]
            };
            let own_shader = shader.is_some() && state.unit_id == *target;
            let source = if (kind == Kind::Sticker && canvas.image.is_some())
                || (own_shader && canvas.editor.layers().is_some())
            {
                let key = format!("document:{index}");
                origins.push(Origin {
                    key: key.clone(),
                    document: index,
                    port: None,
                });
                Some(key)
            } else {
                None
            };
            let mut values = Vec::new();
            if kind == Kind::Art {
                for (output_index, port) in
                    state.execution_results.borrow().ports().iter().enumerate()
                {
                    let key = if port.value.image().is_some() {
                        let key = format!("output:{index}:{output_index}");
                        origins.push(Origin {
                            key: key.clone(),
                            document: index,
                            port: Some(port.name.clone()),
                        });
                        Some(key)
                    } else {
                        None
                    };
                    // 非图像端口保留占位，防止错误回退到另一个图像输出。
                    values.push((port.name.clone(), key));
                }
            }
            snapshots.push(Snapshot {
                id: state.unit_id.clone(),
                kind,
                ports,
                values,
                source,
                edited: canvas.editor.has_edit_state(),
                accept_upstream: state.accept_upstream.get(),
                sticker: state.sticker_input.snapshot(),
            });
        }
        for session in sessions.iter().filter(|session| {
            dependencies.contains(&session.unit_id())
                && session.is_alive()
                && session.belongs_to(workspace)
        }) {
            snapshots.push(Snapshot {
                id: session.unit_id().clone(),
                kind: Kind::Sticker,
                ports: Vec::new(),
                values: Vec::new(),
                source: Some(format!("live:{}", session.unit_id().as_str())),
                edited: false,
                accept_upstream: false,
                sticker: Default::default(),
            });
        }
        let ports: Vec<Vec<Port<'_>>> = snapshots
            .iter()
            .map(|node| {
                node.ports
                    .iter()
                    .map(|(name, kind)| Port { name, kind })
                    .collect()
            })
            .collect();
        let values: Vec<Vec<(&str, Output<'_>)>> = snapshots
            .iter()
            .map(|node| {
                node.values
                    .iter()
                    .map(|(name, key)| {
                        (
                            name.as_str(),
                            key.as_deref().map(Output::Image).unwrap_or(Output::Null),
                        )
                    })
                    .collect()
            })
            .collect();
        let nodes: Vec<_> = snapshots
            .iter()
            .enumerate()
            .map(|(index, node)| Node {
                id: &node.id,
                kind: node.kind,
                inputs: &ports[index],
                outputs: &[],
                capability_inputs: (node.kind == Kind::Art).then_some(ports[index].as_slice()),
                capability_outputs: &[],
                values: &values[index],
                source: node.source.as_deref(),
                preview: None,
                manual_image: node.sticker.asset.as_ref().map(|asset| asset.uri.as_str()),
                input_disabled: node.sticker.disabled,
                accept_upstream: node.accept_upstream,
                locally_edited: node.edited,
                pending: false,
            })
            .collect();
        let graph = Graph::new(&nodes, workspace.links()).map_err(|_| "工作区图片图超过限制")?;
        let target_state = &documents
            .iter()
            .find(|doc| doc.state.unit_id == *target)
            .ok_or("目标窗口已关闭")?
            .state;
        let selected = target_state.selected_art.borrow();
        let manual: Vec<_> = if let Some(resolved) = resolved.filter(|_| shader.is_some()) {
            resolved
                .entries()
                .filter_map(|(name, value)| match value {
                    Some(Output::Image(value)) if value.starts_with("data:") => {
                        Some((name, Output::Image(value)))
                    }
                    _ => None,
                })
                .collect()
        } else {
            selected
                .as_ref()
                .into_iter()
                .flat_map(|selected| selected.parameters.entries())
                .filter_map(|(name, value)| {
                    value
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| value.starts_with("data:"))
                        .map(|value| (name, Output::Image(value)))
                })
                .collect()
        };
        let mut images = graph
            .execution_images(target, &manual)
            .map_err(|_| "无法解析图片输入")?;
        // 旧 Shader controller 最后回退自身 data.src。仅可信 Shader 链可用，不能回读 display/预览。
        let own_input = shader.and_then(|capability| {
            let ports: Vec<_> = images.iter().map(|(name, _)| *name).collect();
            if capability.available_shader_input(&ports).is_some() {
                return None;
            }
            snapshots
                .iter()
                .find(|node| node.id == *target)
                .and_then(|node| node.source.as_deref())
                .map(|key| (capability.shader_input_port().unwrap_or("input"), key))
        });
        if let Some(input) = own_input {
            images
                .try_reserve(1)
                .map_err(|_| "无法分配 Shader 自身图片输入")?;
            images.push(input);
        }
        let prepared = shader_inputs::resolve(shader, &graph, &references, &mut images)?;
        // 图视图包含所有候选，但被连接图片遮蔽的 Unit 不拥有本次来源生命周期。
        let mut dependencies = target_dependencies;
        for unit in prepared.units {
            for dependency in workspace
                .upstream_units(unit)
                .map_err(|_| "无法解析 Shader 参数依赖")?
            {
                if !dependencies.contains(&dependency) {
                    dependencies
                        .try_reserve(1)
                        .map_err(|_| "无法分配图片依赖")?;
                    dependencies.push(dependency);
                }
            }
        }
        let shader_parameters = prepared.parameters;
        if !manual_trigger
            && graph
                .missing_execution_images(target, &manual)
                .map_err(|_| "无法检查缺失图片")?
                .iter()
                .any(|name| {
                    own_input.is_none_or(|(port, _)| port != *name)
                        && !shader_parameters.iter().any(|port| port == name)
                })
        {
            return Ok(None);
        }
        if images.len() > 32 {
            return Err("图片输入数量超过准备预算");
        }
        // 只有手动触发享有缺图豁免；两种触发均不伪造输入。
        let mut names = Vec::new();
        let mut sources = Vec::new();
        // 同一执行中的多个端口共享同一次 Live 帧边界，不混用流的不同时间点。
        let mut live_frames = std::collections::HashMap::new();
        for (name, key) in images {
            if let Some(id) = key.strip_prefix("live:") {
                let session = sessions
                    .iter()
                    .find(|session| {
                        session.unit_id().as_str() == id
                            && session.is_alive()
                            && session.belongs_to(workspace)
                    })
                    .ok_or("Live 来源已停止")?;
                let frame = if let Some(frame) = live_frames.get(id) {
                    std::sync::Arc::clone(frame)
                } else {
                    let Some(frame) = session
                        .snapshot_frame()
                        .map_err(|_| "无法捕获 Live 输入帧")?
                    else {
                        if !manual_trigger {
                            return Ok(None);
                        }
                        return Err("Live 尚无可用画面");
                    };
                    live_frames.insert(id, frame.clone());
                    frame
                };
                sources.push(
                    Source::capture_live(
                        name.into(),
                        session.unit_id().clone(),
                        session.input_identity(),
                        frame,
                    )
                    .map_err(|_| "Live 输入帧超过预算")?,
                );
                names.push(name.into());
                continue;
            }
            if key.starts_with("data:") {
                if let Some(node) = snapshots.iter().find(|node| {
                    node.sticker
                        .asset
                        .as_ref()
                        .is_some_and(|a| std::ptr::eq(a.uri.as_str(), key))
                }) {
                    let state = &documents
                        .iter()
                        .find(|doc| doc.state.unit_id == node.id)
                        .ok_or("手动图片来源已关闭")?
                        .state;
                    let asset = node.sticker.asset.as_ref().unwrap();
                    let source = Source::capture_sticker(
                        name.into(),
                        state.unit_id.clone(),
                        &asset.image,
                        &state.canvas.borrow().editor,
                    )
                    .map_err(|_| "手动图片存在未完成编辑或超过预算")?;
                    names.push(name.into());
                    sources.push(source);
                    continue;
                }
                let canvas = target_state.canvas.borrow();
                let source =
                    Source::capture_inline(name.into(), target.clone(), key, &canvas.editor)
                        .map_err(|_| "内联图片格式不支持或超过预算")?;
                names.push(name.into());
                sources.push(source);
                continue;
            }
            let origin = origins
                .iter()
                .find(|origin| origin.key == key)
                .ok_or("图片引用不是本地资源")?;
            let state = &documents[origin.document].state;
            let source = if let Some(port) = &origin.port {
                let image = state
                    .execution_results
                    .borrow()
                    .image_source(port)
                    .ok_or("上游正式图片已失效")?;
                Source::capture_output(name.into(), state.unit_id.clone(), image)
            } else {
                let canvas = state.canvas.borrow();
                if state.is_art() {
                    let source =
                        Source::capture_base(name.into(), state.unit_id.clone(), &canvas.editor)
                            .map_err(|_| "Shader 自身底图存在未完成编辑或无法捕获")?;
                    names.push(name.into());
                    sources.push(source);
                    continue;
                }
                let image = canvas.image.as_ref().ok_or("来源图片已关闭")?;
                Source::capture(
                    name.into(),
                    state.unit_id.clone(),
                    image.width,
                    image.height,
                    image.pixels.clone(),
                    &canvas.editor,
                )
            }
            .map_err(|_| "图片来源有未完成编辑或无法捕获")?;
            names.push(name.into());
            sources.push(source);
        }
        // 捕获整个上游子图，包括当前停止中继的节点；编辑/能力切换可能使解析改路。
        if let Some(source) = sources.first_mut() {
            for document in documents {
                if !dependencies.contains(&&document.state.unit_id) {
                    continue;
                }
                if document.state.pending_adoption.borrow().is_some() {
                    return Err("图片依赖存在未完成接管");
                }
                source
                    .depend_on(&document.state)
                    .map_err(|_| "工作区存在未完成编辑，无法捕获图片依赖")?;
            }
        }
        Ok(Some(ImageInputs {
            ports: names,
            sources,
            shader_parameters,
        }))
    }
}
