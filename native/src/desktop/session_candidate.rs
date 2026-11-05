//! 拥有原始会话和图像的后台候选；窗口元数据独立，不构造自引用 JSON 对象。
use super::{
    session_asset_root::Root,
    session_prepare::{self, Images},
};
use crate::{session_document, session_window::Rect, workspace::Workspace, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_ACCESSDENIED, E_INVALIDARG},
};
#[path = "session_projection_candidate.rs"]
pub mod projection;
#[path = "session_candidate_results.rs"]
mod results;
#[path = "session_workflow_bake.rs"]
mod workflow_bake;
pub use workflow_bake::workflow::job::archive::{
    Job as WorkflowArchiveJob, Staged as WorkflowArchiveStaged,
};
pub use workflow_bake::workflow::job::sync::{
    Delivery as WorkflowDelivery, Job as WorkflowSyncJob,
};
pub use workflow_bake::workflow::job::{
    Input as WorkflowInput, Job as WorkflowJob, Ready as WorkflowReady,
};

#[derive(Clone)]
pub struct Placement {
    pub capture_meta: Option<String>,
    pub unit: UnitId,
    pub art: bool,
    pub art_id: Option<String>,
    pub local_art_id: Option<String>,
    pub(crate) local_art_ports: Option<crate::art_execution_grant::package::ports::Record>,
    pub rect: Rect,
    pub minified: bool,
    pub minified_views: Option<crate::session_minified::Views>,
    pub saved_rect: Option<Rect>,
    pub crop_offset: Option<crate::session_window::Offset>,
    pub crop: Option<crate::session_crop::Restored>,
    pub opacity_normal: f64,
    pub opacity_mini: f64,
}
pub struct Candidate {
    pub(in crate::desktop) sender_index:
        Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    original: crate::session_source::Source,
    pub(in crate::desktop) projection_index:
        Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub(in crate::desktop) projection_scope: Option<crate::session_projection_scope::Proof>,
    pub origin: Option<Origin>,
    pub schema_version: u32,
    pub revision: u64,
    pub coordinate_space: Option<crate::session_placement::Space>,
    pub workspace: Workspace,
    pub placements: Vec<Placement>,
    pub images: Vec<Images>,
}
pub struct Origin {
    pub requested: std::path::PathBuf,
    pub recovery: Option<std::path::PathBuf>,
}
/// 图片与矢量/效果均已准备的阶段结果；仍不等于完整窗口恢复事务。
pub struct VectorCandidate {
    pub(in crate::desktop) projection_edits: Vec<(UnitId, super::projection_edit_workspace::Owner)>,
    pub candidate: Candidate,
    pub vectors: Vec<(UnitId, crate::session_vectors::Restored)>,
    pub(in crate::desktop) regions: Vec<(UnitId, super::hit_region::HitRegion)>,
}
impl VectorCandidate {
    /// 在工作线程调用；DirectWrite 接口只在测量调用内持有，不随结果跨线程。
    pub fn prepare(candidate: Candidate, cancel: &AtomicBool) -> Result<Self> {
        let mut vectors = candidate.prepare_vectors(cancel)?;
        let document = session_document::decode(candidate.original_json())
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid retained crop source"))?;
        for (unit, restored) in &mut vectors {
            check(cancel)?;
            let placement = candidate
                .placements
                .iter()
                .find(|placement| placement.unit == *unit)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Missing restored editor placement"))?;
            let frame = placement
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid restored editor frame"))?;
            let images = candidate
                .images
                .iter()
                .find(|images| images.unit == *unit)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Missing restored editor images"))?;
            let layers = if let Some(crop) = placement.crop {
                let geometry = crop
                    .editor_crop()
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored raster crop"))?;
                // Canvas 将独立标注映射到可见框；不要装入已为效果计算投影到完整源框的纹理。
                images.prepare_editor_layers(geometry.source, cancel)?
            } else {
                images.prepare_editor_layers(
                    crate::crop_geometry::Size {
                        width: frame.w,
                        height: frame.h,
                    },
                    cancel,
                )?
            };
            if let Some(layers) = layers {
                let editor = std::mem::take(&mut restored.editor);
                restored.editor = editor
                    .with_restored_layers(layers)
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored editor assembly"))?;
                if placement.crop.is_none() {
                    restored.editor = std::mem::take(&mut restored.editor)
                        .with_restored_size(crate::crop_geometry::Size {
                            width: frame.w,
                            height: frame.h,
                        })
                        .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored logical frame"))?;
                }
            }
            if let Some(crop) = placement.crop {
                let geometry = crop
                    .editor_crop()
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored crop view"))?;
                restored.editor = std::mem::take(&mut restored.editor)
                    .with_restored_crop(geometry)
                    .map_err(|_| Error::new(E_INVALIDARG, "Cannot install restored crop view"))?;
            } else if restored.editor.layers().is_some() {
                let window = document
                    .windows
                    .iter()
                    .find(|window| window.id == unit.as_str())
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Missing legacy crop source"))?;
                if let Some(image) = images.source.as_ref() {
                    if let Some(pixels) =
                        crate::session_crop::legacy_pixels(window, (image.width, image.height))
                            .map_err(|_| Error::new(E_INVALIDARG, "Invalid legacy crop source"))?
                    {
                        restored.editor = std::mem::take(&mut restored.editor)
                            .with_restored_legacy_crop(
                                pixels,
                                crate::crop_geometry::Size {
                                    width: image.width as f64,
                                    height: image.height as f64,
                                },
                            )
                            .map_err(|_| {
                                Error::new(E_INVALIDARG, "Cannot install legacy crop view")
                            })?;
                    }
                }
            }
        }
        let projection_edits =
            super::session_projection_edit::prepare(&document, &mut vectors, cancel)?;
        let mut regions = Vec::with_capacity(vectors.len());
        for (unit, restored) in &vectors {
            check(cancel)?;
            let images = candidate
                .images
                .iter()
                .find(|images| images.unit == *unit)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Missing restored hit source"))?;
            let layers = restored.editor.layers();
            let committed = layers.map(|layers| super::image::Image {
                width: layers.size.0,
                height: layers.size.1,
                pixels: layers.base.clone(),
            });
            let minified = candidate
                .placements
                .iter()
                .any(|placement| placement.unit == *unit && placement.minified);
            let image = if minified {
                images.source.as_ref().or(images.preview.as_ref())
            } else {
                committed.as_ref().or(images.display())
            };
            let region = if let Some(image) = image {
                let result = super::hit_region::HitRegion::prepare_cancellable(
                    image,
                    layers.and_then(|layers| layers.annotations.as_ref()),
                    layers.map_or((image.width, image.height), |layers| {
                        layers.annotation_dimensions()
                    }),
                    || cancel.load(Ordering::Acquire),
                );
                check(cancel)?;
                result?
            } else {
                Default::default()
            };
            regions.push((unit.clone(), region));
        }
        check(cancel)?;
        Ok(Self {
            projection_edits,
            candidate,
            vectors,
            regions,
        })
    }
}
impl Placement {
    pub fn expanded_rect(&self) -> Option<Rect> {
        crate::session_window::expanded_rect(
            self.rect,
            self.minified,
            self.saved_rect,
            self.crop_offset,
        )
    }
}
impl Candidate {
    pub(in crate::desktop) fn retained_source_record(&self) -> crate::session_source::Source {
        self.original.clone()
    }
    pub fn original_json(&self) -> &[u8] {
        &self.original
    }
    /// 工作区与窗口已经接管资源后，移动原文留存未知字段，不复制整个会话。
    pub(in crate::desktop) fn into_original_json(self) -> crate::session_source::Source {
        self.original
    }
    /// 基础矢量候选；任何不支持/损坏标注使整批失败，不修改正在显示的文档。
    pub fn prepare_vectors(
        &self,
        cancel: &AtomicBool,
    ) -> Result<Vec<(UnitId, crate::session_vectors::Restored)>> {
        check(cancel)?;
        let document = session_document::decode(&self.original)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid retained session"))?;
        let mut restored = Vec::new();
        for window in &document.windows {
            check(cancel)?;
            let frame = window
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid effect frame"))?;
            let crop = crate::session_crop::restore(window)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid effect crop"))?;
            let images = self
                .images
                .iter()
                .find(|images| images.unit.as_str() == window.id);
            let mut effects = if let Some(restored) = crop {
                super::session_blur_resources::Resources::for_cropped_editor(
                    images, restored, cancel,
                )
            } else {
                super::session_blur_resources::Resources::new(
                    images,
                    crate::crop_geometry::Size {
                        width: frame.w,
                        height: frame.h,
                    },
                    crop.map(|crop| crop.crop),
                    cancel,
                )
            };
            let mut editor = crate::session_vectors::restore_with_resources(
                window.annotation_state,
                |text| {
                    let size = unsafe { super::annotation_text::measure(text) }
                        .map_err(|_| crate::session_vectors::Error::Geometry)?;
                    let metrics = super::annotation_text::measure_flip_metrics(text)
                        .map_err(|_| crate::session_vectors::Error::Geometry)?;
                    Ok(crate::session_vectors::TextMetrics {
                        size,
                        baseline: metrics.baseline_from_top,
                    })
                },
                |strength| effects.get(strength),
            )
            .map_err(|error| {
                Error::new(
                    if error == crate::session_vectors::Error::Cancelled {
                        E_ABORT
                    } else {
                        E_INVALIDARG
                    },
                    format!("Session vector restore rejected: {error:?}"),
                )
            })?;
            if let Some(crop) = crop {
                let geometry = crop
                    .editor_crop()
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored vector crop"))?;
                editor.editor = editor
                    .editor
                    .with_restored_offset(crate::viewport::Point {
                        x: geometry.rect.x as f32,
                        y: geometry.rect.y as f32,
                    })
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored vector offset"))?;
            }
            let unit = UnitId::parse(&window.id)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid session unit"))?;
            restored.push((unit, editor));
        }
        check(cancel)?;
        Ok(restored)
    }
    /// 图片 worker 完成后按当前目录解析；只返回局部候选，不改变自身或活动工作区。
    pub fn resolve_arts(
        &self,
        catalog: Option<&crate::loom_handshake::Handshake>,
        cancel: &AtomicBool,
    ) -> Result<Vec<crate::session_art_batch::Entry>> {
        check(cancel)?;
        let document = session_document::decode(&self.original)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid retained session"))?;
        crate::session_art_batch::resolve_optional(&document, catalog, cancel).map_err(|error| {
            let code = if error == crate::session_art_batch::Error::Cancelled {
                E_ABORT
            } else {
                E_INVALIDARG
            };
            Error::new(code, format!("Session Art preparation rejected: {error:?}"))
        })
    }
}
/// 仅准备拓扑/窗口元数据/图片。参数、编辑历史等仍在 original 中，不能据此发布完整恢复。
pub fn prepare(bytes: Vec<u8>, root: Option<&Root>, cancel: &AtomicBool) -> Result<Candidate> {
    prepare_source(bytes.into(), root, None, cancel)
}
fn prepare_source(
    bytes: crate::session_source::Source,
    root: Option<&Root>,
    images: Option<Vec<Images>>,
    cancel: &AtomicBool,
) -> Result<Candidate> {
    check(cancel)?;
    let document = session_document::decode(&bytes)
        .map_err(|error| Error::new(E_INVALIDARG, format!("Session decode rejected: {error:?}")))?;
    check(cancel)?;
    let images = match images {
        Some(images) => images,
        None => session_prepare::images(&document, cancel, |reference| {
            root.ok_or_else(|| {
                Error::new(
                    E_ACCESSDENIED,
                    "Session file resources require an admitted root",
                )
            })?
            .open_image(reference)
        })?,
    };
    let mut placements = Vec::new();
    for window in &document.windows {
        check(cancel)?;
        let intrinsic = images
            .iter()
            .find(|images| images.unit.as_str() == window.id)
            .and_then(|images| images.source.as_ref().or(images.preview.as_ref()))
            .map(|image| crate::crop_geometry::Size {
                width: image.width as f64,
                height: image.height as f64,
            });
        placements.push(Placement {
            capture_meta: window.capture_meta.map(|value| value.get().to_owned()),
            unit: UnitId::parse(&window.id)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid session unit"))?,
            art: window.is_art(),
            art_id: window.art_id.clone(),
            local_art_ports: window
                .otter_native_local_art_ports
                .map(|raw| {
                    if !window.otter_native_local_art || raw.get().len() > 16 * 1024 {
                        return Err(Error::new(E_INVALIDARG, "Invalid local Art port record"));
                    }
                    let record: crate::art_execution_grant::package::ports::Record =
                        serde_json::from_str(raw.get()).map_err(|_| {
                            Error::new(E_INVALIDARG, "Invalid local Art port record")
                        })?;
                    record
                        .validate(window.art_id.as_deref().unwrap_or_default())
                        .map_err(|_| Error::new(E_INVALIDARG, "Invalid local Art port binding"))?;
                    Ok(record)
                })
                .transpose()?,
            local_art_id: if window.otter_native_local_art {
                Some(
                    window
                        .art_id
                        .as_ref()
                        .filter(|id| {
                            window.is_art()
                                && !id.is_empty()
                                && id.len() <= 256
                                && !id.chars().any(char::is_control)
                        })
                        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid local Art identity"))?
                        .clone(),
                )
            } else {
                None
            },
            rect: window.rect(),
            minified: window.is_minified(),
            minified_views: crate::session_minified::restore(window, intrinsic)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored minified viewport"))?,
            saved_rect: window.saved_rect,
            crop_offset: window.crop_offset,
            crop: crate::session_crop::restore(window)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored crop geometry"))?,
            opacity_normal: window.opacity(false),
            opacity_mini: window.opacity(true),
        });
    }
    check(cancel)?;
    Ok(Candidate {
        projection_index: crate::projection_delivery::binding::checkpoint::Index::capture(&bytes),
        sender_index: crate::projection_delivery::sender::binding::checkpoint::Index::capture(
            &bytes,
        ),
        projection_scope: crate::session_projection_scope::Proof::capture(&bytes),
        origin: None,
        schema_version: document.schema_version,
        revision: document.revision,
        coordinate_space: document.coordinate_space,
        workspace: document.workspace,
        placements,
        images,
        original: bytes,
    })
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Session preparation cancelled"))
    } else {
        Ok(())
    }
}
