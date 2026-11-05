//! 冻结工作区到整会话候选；内容编码由调用方提供，禁止把几何候选直接提交磁盘。
use super::{Data, Document};
use crate::{
    session_write::{self, Candidate, Error, WindowCandidate},
    workspace_session::{SessionGroup, SessionLink, Sticker, Topology},
};
use serde_json::{Map, Value};
use std::sync::atomic::AtomicBool;

impl Data {
    /// 后台完整内容候选入口；仍须在 UI 复核 Receipt 并由磁盘事务提交，不能直接发布。
    pub fn encode(mut self, cancel: &AtomicBool) -> Result<Candidate, Error> {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let _apartment = crate::desktop::Apartment::new().map_err(|_| Error::Window)?;
        for live in std::mem::take(&mut self.live) {
            self.documents.push(live.materialize(cancel)?);
        }
        let space = self.space;
        if self
            .transient_host
            .as_ref()
            .is_some_and(|host| self.projection_edits.iter().any(|(unit, _)| unit == host))
        {
            self.transient_host = None;
        }
        let mut projection_edits = std::mem::take(&mut self.projection_edits);
        self.assemble(cancel, |doc, fields| {
            let projection_edit = projection_edits
                .iter()
                .position(|(unit, _)| *unit == doc.unit)
                .map(|index| {
                    projection_edits
                        .swap_remove(index)
                        .1
                        .for_save(doc.annotation_mapping(&fields, space)?)
                        .map_err(|_| Error::Window)
                })
                .transpose()?;
            let fields = doc.write_annotations(fields, space, cancel)?;
            let original = doc
                .capture_original
                .as_ref()
                .map(|file| crate::capture_file::encode_inline(file, cancel))
                .transpose()
                .map_err(|error| {
                    if error == crate::loom_base64::Error::Cancelled {
                        Error::Cancelled
                    } else {
                        Error::Window
                    }
                })?;
            let images = doc.encode_images(cancel).map_err(|_| {
                if cancel.load(std::sync::atomic::Ordering::Acquire) {
                    Error::Cancelled
                } else {
                    Error::Window
                }
            })?;
            let mut candidate = match images {
                Some(images) => images
                    .into_data_urls(cancel)
                    .map_err(|_| {
                        if cancel.load(std::sync::atomic::Ordering::Acquire) {
                            Error::Cancelled
                        } else {
                            Error::Window
                        }
                    })?
                    .install(fields),
                None => Ok(WindowCandidate::new(fields)),
            }?;
            if let Some(original) = original {
                candidate.insert_encoded("otterNativeCaptureOriginalSrc", original);
            }
            for (name, image) in doc.output_images {
                let png = image.png(cancel).map_err(|_| {
                    if cancel.load(std::sync::atomic::Ordering::Acquire) {
                        Error::Cancelled
                    } else {
                        Error::Window
                    }
                })?;
                let encoded =
                    crate::loom_base64::encode_png_data_url(&png, cancel).map_err(|error| {
                        if error == crate::loom_base64::Error::Cancelled {
                            Error::Cancelled
                        } else {
                            Error::Budget
                        }
                    })?;
                candidate.insert_encoded_output(&name, encoded)?;
            }
            candidate.write_projection_edit(projection_edit.as_ref())?;
            Ok(candidate)
        })
    }
    /// 仅跳过仍为空的初始宿主。曾编辑、钉图、分组、连线或选择 Art 后必须成为保存对象。
    fn omit_host(&self, doc: &Document) -> bool {
        self.transient_host.as_ref() == Some(&doc.unit)
            && !self
                .port_visibility
                .iter()
                .any(|(unit, _)| unit == &doc.unit)
            && !doc.pinned
            && !doc.art
            && doc.group.is_none()
            && doc.parameters.is_none()
            && doc
                .sticker
                .as_ref()
                .is_none_or(|(config, accept)| !config.configured() && *accept)
            && doc.execution.is_none()
            && doc.outputs.is_none()
            && doc.output_images.is_empty()
            && doc.editor.revision() == 0
            && doc.editor.elements().is_empty()
            && doc.editor.layers().is_none()
            && doc.editor.crop_view().is_none()
            && doc.restored_source.is_none()
            && doc.local_image.is_none()
            && doc.capture_meta.is_none()
            && doc.capture_original.is_none()
            && !self
                .projection_edits
                .iter()
                .any(|(unit, _)| *unit == doc.unit)
            && !self
                .links
                .iter()
                .any(|link| link.from_unit == doc.unit || link.to_unit == doc.unit)
    }

    /// content 必须完成图片、裁剪和标注编码；它消费快照，不能访问活动 HWND 或发布副作用。
    pub fn assemble(
        self,
        cancel: &AtomicBool,
        mut content: impl FnMut(Document, Map<String, Value>) -> Result<WindowCandidate, Error>,
    ) -> Result<Candidate, Error> {
        if !self.live.is_empty() {
            return Err(Error::Window);
        }
        let topology = Topology {
            stickers: self
                .documents
                .iter()
                .filter(|doc| !self.omit_host(doc))
                .map(|doc| Sticker {
                    id: doc.unit.as_str().into(),
                    group_id: doc.group.as_ref().map(|id| id.as_str().into()),
                })
                .collect(),
            links: self
                .links
                .iter()
                .map(|link| SessionLink {
                    id: link.id.as_str().into(),
                    from_unit_id: link.from_unit.as_str().into(),
                    from_port_id: link.from_port.clone(),
                    to_unit_id: link.to_unit.as_str().into(),
                    to_port_id: link.to_port.clone(),
                })
                .collect(),
            groups: self
                .groups
                .iter()
                .map(|group| SessionGroup {
                    id: group.id.as_str().into(),
                    name: group.name.clone(),
                    hidden: group.hidden,
                    locked: group.locked,
                })
                .collect(),
        };
        // 先验证原始列表，不能让 HashMap 的覆盖语义掩盖重复窗口身份。
        let mut ids = std::collections::HashSet::new();
        if self
            .documents
            .iter()
            .any(|doc| !ids.insert(doc.unit.clone()))
        {
            return Err(Error::Topology);
        }
        let mut documents: std::collections::HashMap<_, _> = self
            .documents
            .into_iter()
            .map(|doc| (doc.unit.as_str().to_owned(), doc))
            .collect();
        session_write::assemble_owned_at_revision(
            &self.original,
            &topology,
            cancel,
            Some(self.space),
            self.disk_revision,
            Some(self.projection_scope),
            |sticker, original| {
                let mut doc = documents.remove(&sticker.id).ok_or(Error::Topology)?;
                let geometry = doc.geometry().map_err(|_| Error::Window)?;
                let Value::Object(mut fields) =
                    crate::session_window_write::merge(original.into(), geometry, self.space)
                        .map_err(|_| Error::Window)?
                else {
                    return Err(Error::Window);
                };
                // 缺少当前会话owner时不能把原文中的旧快照原样带回，避免删除/reset后复活。
                fields.remove(crate::projection_edit::workspace::FIELD);
                if let Some((_, visibility)) = self
                    .port_visibility
                    .iter()
                    .find(|(unit, _)| unit == &doc.unit)
                {
                    fields.insert(
                        "portVisibility".into(),
                        serde_json::to_value(visibility).map_err(|_| Error::Window)?,
                    );
                } else {
                    fields.remove("portVisibility");
                }
                if let Some(outputs) = doc.outputs.take() {
                    fields.insert("outputs".into(), Value::Object(outputs));
                }
                if doc.capture_original.is_some()
                    || doc
                        .capture_meta
                        .as_ref()
                        .and_then(|metadata| metadata.get("dynamicRange"))
                        .and_then(Value::as_str)
                        != Some("hdr")
                {
                    fields.remove("otterNativeCaptureOriginalSrc");
                }
                if let Some(metadata) = doc.capture_meta.take() {
                    let raw = serde_json::to_string(&metadata).map_err(|_| Error::Window)?;
                    crate::session_capture::decode(&raw).map_err(|_| Error::Window)?;
                    fields.insert("captureMeta".into(), metadata);
                } else {
                    fields.remove("captureMeta");
                }
                let previous_crop = fields.remove("imageEditState");
                let crop = if doc.restored_source.is_some() {
                    doc.editor.crop_view()
                } else {
                    doc.editor.raster_crop_view()
                };
                if previous_crop.is_some() || crop.is_some() {
                    let state = crate::session_crop_write::merge_view(previous_crop, crop)
                        .map_err(|_| Error::Window)?;
                    fields.insert("imageEditState".into(), state);
                }
                match (&doc.parameters, doc.execution) {
                    (Some(parameters), Some(execution)) if doc.art => {
                        fields.remove("otterNativeLocalArt");
                        fields.remove("otterNativeLocalArtPorts");
                        fields = crate::session_art_write::merge(fields, parameters, execution);
                    }
                    (None, None) => {
                        if let Some(id) = &doc.local_art_id {
                            if !doc.art
                                || id.is_empty()
                                || id.len() > 256
                                || id.chars().any(char::is_control)
                            {
                                return Err(Error::Window);
                            }
                            fields.insert("type".into(), "art".into());
                            fields.insert("artId".into(), id.clone().into());
                            fields.insert("otterNativeLocalArt".into(), true.into());
                            if let Some(ports) = &doc.local_art_ports {
                                ports.validate(id).map_err(|_| Error::Window)?;
                                fields.insert(
                                    "otterNativeLocalArtPorts".into(),
                                    serde_json::to_value(ports).map_err(|_| Error::Window)?,
                                );
                            } else {
                                fields.remove("otterNativeLocalArtPorts");
                            }
                        }
                        if doc.art && fields.get("type").and_then(Value::as_str) != Some("art") {
                            return Err(Error::Window);
                        }
                        if !doc.art {
                            fields.remove("otterNativeLocalArt");
                            fields.remove("otterNativeLocalArtPorts");
                            for key in ["artId", "executionConfig"] {
                                fields.remove(key);
                            }
                            fields.insert("type".into(), "sticker".into());
                            let (config, accept) = doc.sticker.clone().unwrap_or_default();
                            let params = config.parameters();
                            crate::sticker_image_params::validate(&params)
                                .map_err(|_| Error::Window)?;
                            fields.insert("params".into(), Value::Object(params));
                            let propagation = fields
                                .entry("stickerEditPropagation")
                                .or_insert_with(|| Value::Object(Map::new()));
                            let propagation = propagation.as_object_mut().ok_or(Error::Window)?;
                            propagation.insert(
                                "acceptUpstream".into(),
                                if doc.sticker.is_some() { accept } else { true }.into(),
                            );
                            propagation
                                .insert("locallyEdited".into(), doc.editor.has_edit_state().into());
                        }
                    }
                    _ => return Err(Error::Window),
                }
                content(doc, fields)
            },
        )
    }
}

#[cfg(test)]
#[path = "application_save_output_image_tests.rs"]
mod output_image_tests;
#[cfg(test)]
#[path = "application_save_ports_tests.rs"]
mod ports_tests;
#[cfg(test)]
#[path = "application_save_roundtrip_tests.rs"]
mod roundtrip_tests;
#[cfg(test)]
#[path = "application_save_sticker_tests.rs"]
mod sticker_tests;
#[cfg(test)]
#[path = "application_save_assemble_tests.rs"]
mod tests;
