//! 工作区显示解析到共享像素/编辑快照；不覆盖 Canvas 的原图，不创建 GPU 资源或编码图片。
use super::Application;
#[path = "application_display_cache.rs"]
mod cache;
#[path = "application_display_live.rs"]
mod live;
use crate::{
    desktop::{display_frame::Frame, image::Image},
    graph_image::{Graph, Kind, Node, Output, Port},
};
pub(super) use cache::Cache;
struct Origin {
    key: String,
    owner: Owner,
    local: bool,
    image: Image,
}
enum Owner {
    Document(usize),
    Live(crate::workspace_id::UnitId),
}
struct View {
    source: Option<String>,
    preview: Option<String>,
    values: Vec<(String, Option<String>)>,
    art: bool,
    edited: bool,
    accept: bool,
}
impl Application {
    pub(super) fn sync_display_frames(&self) {
        if self.display_composition.borrow_mut().poll() {
            self.display_cache.borrow_mut().take();
        }
        let (live_images, live_pending) = self.live_display_images();
        if !live_pending
            && self
                .display_cache
                .borrow()
                .as_ref()
                .is_some_and(|cache| cache.matches(&self.documents, &self.workspace))
        {
            return;
        }
        self.display_composition
            .borrow_mut()
            .retain(|id, image, stamp| {
                self.documents
                    .iter()
                    .find(|doc| doc.state.unit_id == *id)
                    .is_some_and(|doc| {
                        let canvas = doc.state.canvas.borrow();
                        stamp.matches(&canvas.editor)
                            && canvas.editor.layers().map_or_else(
                                || {
                                    canvas.image.as_ref().is_some_and(|current| {
                                        current.width == image.width
                                            && current.height == image.height
                                            && current.pixels.same_storage(&image.pixels)
                                    })
                                },
                                |layers| {
                                    layers.size == (image.width, image.height)
                                        && layers.base.same_storage(&image.pixels)
                                },
                            )
                    })
            });
        let mut origins = Vec::new();
        self.sync_parameter_views();
        self.sync_link_menus();
        let mut views = Vec::new();
        let mut pending_composition = live_pending;
        let mut composition_sources = Vec::new();
        for (index, doc) in self.documents.iter().enumerate() {
            let canvas = doc.state.canvas.borrow();
            let art = doc.state.is_art();
            let image = canvas
                .editor
                .layers()
                .map(|layers| Image {
                    width: layers.size.0,
                    height: layers.size.1,
                    pixels: layers.base.clone(),
                })
                .or_else(|| canvas.image.clone());
            let source = image.map(|image| {
                let key = format!("local:{index}");
                origins.push(Origin {
                    key: key.clone(),
                    owner: Owner::Document(index),
                    local: true,
                    image,
                });
                key
            });
            let mut values = Vec::new();
            let mut preview = if canvas.editor.has_edit_state() {
                source.clone()
            } else {
                None
            };
            if art {
                let results = doc.state.execution_results.borrow();
                preview = results.display().map(|image| {
                    let key = format!("display:{index}");
                    origins.push(Origin {
                        key: key.clone(),
                        owner: Owner::Document(index),
                        local: false,
                        image: image.clone(),
                    });
                    key
                });
                for (port_index, port) in results.ports().iter().enumerate() {
                    let value = if let Some(image) = port.value.image() {
                        let key = format!("output:{index}:{port_index}");
                        origins.push(Origin {
                            key: key.clone(),
                            owner: Owner::Document(index),
                            local: false,
                            image: image.clone(),
                        });
                        Some(key)
                    } else {
                        None
                    };
                    values.push((port.name.clone(), value));
                }
            }
            views.push(View {
                source,
                preview,
                values,
                art,
                edited: canvas.editor.has_edit_state(),
                accept: doc.state.accept_upstream.get(),
            });
        }
        let values: Vec<Vec<_>> = views
            .iter()
            .map(|view| {
                view.values
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
        let ports = [Port {
            name: "image",
            kind: "image",
        }];
        let mut nodes: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(index, view)| Node {
                id: &self.documents[index].state.unit_id,
                kind: if view.art { Kind::Art } else { Kind::Sticker },
                inputs: &ports,
                outputs: &[],
                capability_inputs: None,
                capability_outputs: &[],
                values: &values[index],
                source: view.source.as_deref(),
                preview: view.preview.as_deref(),
                manual_image: None,
                input_disabled: self.documents[index]
                    .state
                    .sticker_input
                    .snapshot()
                    .disabled,
                accept_upstream: view.accept,
                locally_edited: view.edited,
                pending: false,
            })
            .collect();
        for entry in &live_images {
            origins.push(Origin {
                key: entry.key.clone(),
                owner: Owner::Live(entry.unit.clone()),
                local: false,
                image: entry.image.clone(),
            });
            nodes.push(Node {
                id: &entry.unit,
                kind: Kind::Sticker,
                inputs: &[],
                outputs: &[],
                capability_inputs: None,
                capability_outputs: &[],
                values: &[],
                source: Some(&entry.key),
                preview: None,
                manual_image: None,
                input_disabled: false,
                accept_upstream: false,
                locally_edited: false,
                pending: false,
            });
        }
        let Ok(graph) = Graph::new(&nodes, self.workspace.links()) else {
            return;
        };
        for (index, doc) in self.documents.iter().enumerate() {
            if doc.state.pending_adoption.borrow().is_some() {
                continue;
            }
            let origin = graph
                .canvas_image(&doc.state.unit_id)
                .and_then(|key| origins.iter().find(|origin| origin.key == key));
            let mut frame = doc.state.resolved_display.borrow_mut();
            let Some(origin) = origin.filter(|origin| !matches!(&origin.owner, Owner::Document(source) if origin.local && *source == index))
            else {
                let changed = frame.take().is_some();
                drop(frame);
                if changed {
                    invalidate(doc);
                }
                continue;
            };
            let source_index = match &origin.owner {
                Owner::Document(index) => *index,
                Owner::Live(unit) => {
                    drop(frame);
                    live::publish(doc, unit, &origin.image, views[index].art);
                    continue;
                }
            };
            let source = &self.documents[source_index].state;
            let composed;
            let image = if views[index].art && origin.local {
                composition_sources.push(source.unit_id.clone());
                match self.display_composition.borrow_mut().request(
                    &source.unit_id,
                    &origin.image,
                    &source.canvas.borrow().editor,
                    self.execution_wake.callback(),
                ) {
                    Ok(Some(image)) => {
                        composed = image;
                        &composed
                    }
                    Ok(None) => {
                        pending_composition = true;
                        continue;
                    }
                    Err(()) => continue,
                }
            } else {
                &origin.image
            };
            // 正式图片可以来自上游，但 Art 覆盖层始终由目标文档拥有。
            // 本地编辑来源已在 worker 合成；此处只叠加目标自己的标注。
            let art_formal = views[index].art;
            let canvas = if art_formal { &doc.state } else { source }.canvas.borrow();
            // Art 的本地标注属于显示覆盖层，不写入正式端口，也不沿正式结果中继传播。
            let own_art_overlay =
                art_formal && (canvas.editor.has_edit_state() || source_index != index);
            let editor = (origin.local || own_art_overlay).then_some(&canvas.editor);
            if !frame
                .as_ref()
                .is_some_and(|frame| frame.matches(&source.unit_id, image, editor, own_art_overlay))
            {
                *frame = Some(if own_art_overlay {
                    match Frame::capture_overlay(
                        source.unit_id.clone(),
                        doc.state.unit_id.clone(),
                        image.clone(),
                        &canvas.editor,
                    ) {
                        Ok(next) => next,
                        Err(error) => {
                            // 保留上一帧；仅在文档或结果改变后重试，避免失败重绘循环。
                            eprintln!("Art overlay mapping failed: {error:?}");
                            continue;
                        }
                    }
                } else {
                    Frame::capture(source.unit_id.clone(), image.clone(), editor)
                });
                drop(frame);
                drop(canvas);
                invalidate(doc);
            }
        }
        // 连接撤销或目标关闭后，不因来源文档仍存活而长期保留合成像素。
        self.display_composition
            .borrow_mut()
            .retain(|id, _, _| composition_sources.contains(id));
        *self.display_cache.borrow_mut() =
            (!pending_composition).then(|| Cache::capture(&self.documents, &self.workspace));
    }
}

fn invalidate(doc: &crate::desktop::document_window::DocumentWindow) {
    // 不同来源可能具有相同的像素身份和局部 revision；换源时退休旧命中缓存。
    *doc.state.hit_region.borrow_mut() = Default::default();
    // 只排队失效区域，不同步发送绘制消息；重复版本不触发绘制，也不擦背景。
    unsafe {
        if doc.state.canvas.borrow().pinned {
            crate::desktop::pin_window::fit(doc.window.0, &doc.state);
        }
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
    }
}
