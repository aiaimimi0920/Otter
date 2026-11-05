//! 保存会话的合成底图选择；借用图像引用，不读取文件、不复制解析出的图片字符串。
use super::{check, Error};
use crate::{
    graph_image::{Graph, Kind, Node, Output, Port},
    loom_capability::Capability,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::atomic::AtomicBool;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Propagation {
    accept_upstream: Option<bool>,
    locally_edited: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Flags {
    id: String,
    sticker_edit_propagation: Option<Propagation>,
}
#[derive(Deserialize)]
struct FlagsDocument {
    stickers: Vec<Flags>,
}

/// 消费回调结束前引用有效；不把借用路径转成跨阶段的读取授权。
pub fn with_sources<T>(
    saved: &[u8],
    catalog: &[Value],
    cancel: &AtomicBool,
    consume: impl FnOnce(&crate::session_document::Document<'_>, &[Option<&str>]) -> Result<T, Error>,
) -> Result<T, Error> {
    check(cancel)?;
    let document = crate::session_document::decode(saved).map_err(|_| Error::Session)?;
    let flags: FlagsDocument = serde_json::from_slice(saved).map_err(|_| Error::Content)?;
    if flags.stickers.len() != document.windows.len() {
        return Err(Error::Node);
    }
    // 参数/正式输出可能携带 inline 图片；为此次额外 JSON 视图保留有界字节预算。
    let bytes = document
        .windows
        .iter()
        .try_fold(0usize, |sum, window| {
            sum.checked_add(window.params.map_or(0, |v| v.get().len()))?
                .checked_add(window.outputs.map_or(0, |v| v.get().len()))
        })
        .ok_or(Error::Budget)?;
    let _lease = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
    let mut parameters = Vec::new();
    let mut outputs = Vec::new();
    let mut inputs = Vec::new();
    let mut capability_outputs = Vec::new();
    let capabilities: Vec<_> = document
        .windows
        .iter()
        .map(|window| {
            window
                .art_id
                .as_deref()
                .and_then(|id| catalog.iter().find(|v| v["id"].as_str() == Some(id)))
                .and_then(Capability::new)
        })
        .collect();
    for (index, (window, flags)) in document.windows.iter().zip(&flags.stickers).enumerate() {
        check(cancel)?;
        if window.id != flags.id {
            return Err(Error::Node);
        }
        parameters.push(super::content::parameters(window, catalog)?);
        let values: Map<String, Value> = match window.outputs {
            Some(raw) if raw.get() != "null" => {
                serde_json::from_str(raw.get()).map_err(|_| Error::Content)?
            }
            _ => Map::new(),
        };
        if values.len() > crate::graph_image::MAX_PORTS {
            return Err(Error::Budget);
        }
        outputs.push(values);
        let capability = capabilities[index].as_ref();
        let ports = if window.is_art() {
            capability
                .as_ref()
                .filter(|c| c.has_input_definitions())
                .map(|c| {
                    // 图解析用完整能力输入，不使用面板可见端口过滤。
                    c.inputs()
                        .take(crate::graph_image::MAX_PORTS + 1)
                        .map(|p| Port {
                            name: p.name,
                            kind: p.kind,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    vec![Port {
                        name: "input_image",
                        kind: "image",
                    }]
                })
        } else {
            vec![Port {
                name: "image",
                kind: "image",
            }]
        };
        inputs.push(ports);
        capability_outputs.push(
            capability
                .as_ref()
                .map(|c| {
                    c.outputs()
                        .take(crate::graph_image::MAX_PORTS + 1)
                        .map(|p| Port {
                            name: p.name,
                            kind: p.kind,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        );
    }
    let values: Vec<Vec<_>> = outputs
        .iter()
        .map(|v| {
            v.iter()
                .map(|(k, v)| (k.as_str(), Output::from_json(v)))
                .collect()
        })
        .collect();
    let nodes: Vec<_> = document
        .windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            let flags = flags.stickers[index].sticker_edit_propagation.as_ref();
            Node {
                id: &document.workspace.units()[index],
                kind: if window.is_art() {
                    Kind::Art
                } else {
                    Kind::Sticker
                },
                inputs: &inputs[index],
                outputs: &[],
                capability_inputs: window.is_art().then_some(inputs[index].as_slice()),
                capability_outputs: &capability_outputs[index],
                values: &values[index],
                source: window.src.as_deref(),
                preview: window.preview_src.as_deref(),
                manual_image: parameters[index].get("image_path").and_then(Value::as_str),
                input_disabled: parameters[index].get("image").and_then(Value::as_str)
                    == Some(crate::graph_params::DISABLED),
                accept_upstream: flags.and_then(|p| p.accept_upstream) != Some(false),
                locally_edited: flags.and_then(|p| p.locally_edited) == Some(true),
                pending: false,
            }
        })
        .collect();
    let graph = Graph::new(&nodes, document.workspace.links()).map_err(|_| Error::Content)?;
    let sources = document
        .windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            let display = graph.image(nodes[index].id);
            let own = window.src.as_deref().filter(|v| !v.is_empty());
            let preview = window.preview_src.as_deref().filter(|v| !v.is_empty());
            if window
                .rasterized_annotation_layer_src
                .as_deref()
                .is_some_and(|v| !v.is_empty())
            {
                own.or(display).or(preview)
            } else {
                display.or(preview).or(own)
            }
        })
        .collect::<Vec<_>>();
    check(cancel)?;
    consume(&document, &sources)
}

#[cfg(test)]
#[path = "session_workflow_sources_tests.rs"]
mod tests;
