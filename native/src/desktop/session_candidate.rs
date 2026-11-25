//! 拥有原始会话和图像的后台候选；窗口元数据独立，不构造自引用 JSON 对象。
use super::{
    session_asset_root::Root,
    session_prepare::{self, Images},
};
use crate::{session_document, session_window::Rect, workspace::Workspace, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_ACCESSDENIED, E_INVALIDARG, E_OUTOFMEMORY},
};
#[path = "session_account_projection_candidate.rs"]
pub mod account_projection;
#[path = "session_candidate_placements.rs"]
mod placements;
#[path = "session_projection_candidate.rs"]
pub mod projection;
#[path = "session_candidate_results.rs"]
mod results;
pub use results::PreparedRestore;
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
    pub node_settings: crate::unit_node_settings::Settings,
    pub(crate) extension_state: crate::unit_attachment_store::Store,
    pub(crate) extension_propagation: Option<std::sync::Arc<serde_json::value::RawValue>>,
    pub(crate) legacy_text: crate::sticker_extra_text::Legacy,
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
    pub rasterized_full: bool,
    pub opacity_normal: f64,
    pub opacity_mini: f64,
    pub frame_style: crate::pin_frame_style::Style,
    pub(crate) mirror: crate::editor::MirrorState,
}
pub struct Candidate {
    pub(in crate::desktop) account_index: Option<crate::projection_account::scene::Index>,
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
        let document = candidate.decode_retained(cancel)?;
        Ok(
            VectorResources::prepare(&candidate.placements, &candidate.images, &document, cancel)?
                .with_candidate(candidate),
        )
    }
}

// 只含拥有型资源，允许各准备阶段在移交 Candidate 前共用借用解析。
struct VectorResources {
    projection_edits: Vec<(UnitId, super::projection_edit_workspace::Owner)>,
    vectors: Vec<(UnitId, crate::session_vectors::Restored)>,
    regions: Vec<(UnitId, super::hit_region::HitRegion)>,
}
impl VectorResources {
    fn prepare(
        placements: &[Placement],
        images: &[Images],
        document: &session_document::Document<'_>,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let mut vectors = Candidate::prepare_vectors_from(placements, images, document, cancel)?;
        // 矢量准备已校验密集批次等长、顺序和身份；后续装配不再按 ID 搜索。
        for (((window, (_, restored)), placement), images) in document
            .windows
            .iter()
            .zip(&mut vectors)
            .zip(placements)
            .zip(images)
        {
            check(cancel)?;
            let frame = placement
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid restored editor frame"))?;
            let geometry = placement
                .crop
                .map(|crop| crop.editor_crop())
                .transpose()
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored raster crop"))?;
            let size = crate::crop_geometry::Size {
                width: frame.w,
                height: frame.h,
            };
            let layers = if let Some(geometry) = geometry {
                // Canvas 将独立标注映射到可见框；不要装入已为效果计算投影到完整源框的纹理。
                images.prepare_full_crop_layers(
                    geometry,
                    placement.crop.is_some_and(|crop| crop.rasterized_source),
                    cancel,
                )?
            } else if placement.rasterized_full {
                images.prepare_rasterized_full_layers(size, cancel)?
            } else {
                images.prepare_editor_layers(size, cancel)?
            };
            if let Some(layers) = layers {
                let editor = std::mem::take(&mut restored.editor);
                restored.editor = editor
                    .with_restored_layers(layers)
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored editor assembly"))?;
                if placement.crop.is_none() {
                    let editor = std::mem::take(&mut restored.editor);
                    restored.editor = if placement.rasterized_full {
                        editor.with_restored_working_size(size)
                    } else {
                        editor.with_restored_size(size)
                    }
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored logical frame"))?;
                }
            }
            if let Some(geometry) = geometry {
                restored.editor = std::mem::take(&mut restored.editor)
                    .with_restored_crop(geometry)
                    .map_err(|_| Error::new(E_INVALIDARG, "Cannot install restored crop view"))?;
            } else if restored.editor.layers().is_some() {
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
            restored.editor = std::mem::take(&mut restored.editor)
                .with_restored_mirror(placement.mirror)
                .map_err(|_| Error::new(E_INVALIDARG, "Cannot install restored mirror state"))?;
        }
        let projection_edits =
            super::session_projection_edit::prepare(document, &mut vectors, cancel)?;
        let mut regions = Vec::with_capacity(vectors.len());
        for (((unit, restored), placement), images) in vectors.iter().zip(placements).zip(images) {
            check(cancel)?;
            let layers = restored.editor.layers();
            let committed = layers.map(|layers| super::image::Image {
                width: layers.size.0,
                height: layers.size.1,
                pixels: layers.base.clone(),
            });
            let image = if placement.minified {
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
            vectors,
            regions,
        })
    }

    fn with_candidate(self, candidate: Candidate) -> VectorCandidate {
        VectorCandidate {
            projection_edits: self.projection_edits,
            vectors: self.vectors,
            regions: self.regions,
            candidate,
        }
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
    fn decode_retained(&self, cancel: &AtomicBool) -> Result<session_document::Document<'_>> {
        check(cancel)?;
        let document = session_document::decode(self.original_json())
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid retained session"))?;
        check(cancel)?;
        Ok(document)
    }
    #[cfg(test)]
    fn prepare_vectors(
        &self,
        cancel: &AtomicBool,
    ) -> Result<Vec<(UnitId, crate::session_vectors::Restored)>> {
        Self::prepare_vectors_from(
            &self.placements,
            &self.images,
            &self.decode_retained(cancel)?,
            cancel,
        )
    }
    /// 基础矢量候选；任何不支持/损坏标注使整批失败，不修改正在显示的文档。
    fn prepare_vectors_from(
        placements: &[Placement],
        images: &[Images],
        document: &session_document::Document<'_>,
        cancel: &AtomicBool,
    ) -> Result<Vec<(UnitId, crate::session_vectors::Restored)>> {
        if document.windows.len() != placements.len() || placements.len() != images.len() {
            return Err(Error::new(E_INVALIDARG, "Session vector count mismatch"));
        }
        let mut restored = Vec::with_capacity(document.windows.len());
        for ((window, placement), images) in document.windows.iter().zip(placements).zip(images) {
            check(cancel)?;
            if window.id != placement.unit.as_str() || images.unit != placement.unit {
                return Err(Error::new(E_INVALIDARG, "Session vector identity mismatch"));
            }
            let frame = placement
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid effect frame"))?;
            let crop = placement.crop;
            let mut effects = if let Some(restored) = crop {
                super::session_blur_resources::Resources::for_cropped_editor(
                    Some(images),
                    restored,
                    cancel,
                )
            } else if placement.rasterized_full {
                super::session_blur_resources::Resources::for_full_working_grid(
                    images,
                    crate::crop_geometry::Size {
                        width: frame.w,
                        height: frame.h,
                    },
                    cancel,
                )
            } else {
                super::session_blur_resources::Resources::new(
                    Some(images),
                    crate::crop_geometry::Size {
                        width: frame.w,
                        height: frame.h,
                    },
                    None,
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
                    match error {
                        crate::session_vectors::Error::Cancelled => E_ABORT,
                        crate::session_vectors::Error::EffectBudget => E_OUTOFMEMORY,
                        _ => E_INVALIDARG,
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
            restored.push((placement.unit.clone(), editor));
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
        Self::resolve_arts_from(&self.decode_retained(cancel)?, catalog, cancel)
    }

    fn resolve_arts_from(
        document: &session_document::Document<'_>,
        catalog: Option<&crate::loom_handshake::Handshake>,
        cancel: &AtomicBool,
    ) -> Result<Vec<crate::session_art_batch::Entry>> {
        crate::session_art_batch::resolve_optional(document, catalog, cancel).map_err(|error| {
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
    prepare_source(&bytes.into(), root, cancel)
}

fn prepare_source(
    source: &crate::session_source::Source,
    root: Option<&Root>,
    cancel: &AtomicBool,
) -> Result<Candidate> {
    let parsed = parse_source(source, cancel)?;
    let (placements, images) = prepare_inputs(parsed.document(), root, cancel)?;
    finish_source(parsed, placements, images, cancel)
}

fn prepare_inputs(
    document: &session_document::Document<'_>,
    root: Option<&Root>,
    cancel: &AtomicBool,
) -> Result<(Vec<Placement>, Vec<Images>)> {
    let images = session_prepare::images(document, cancel, |reference| {
        root.ok_or_else(|| {
            Error::new(
                E_ACCESSDENIED,
                "Session file resources require an admitted root",
            )
        })?
        .open_image(reference)
    })?;
    let placements = placements::prepare(document, &images, cancel)?;
    Ok((placements, images))
}
fn parse_source<'a>(
    source: &'a crate::session_source::Source,
    cancel: &AtomicBool,
) -> Result<crate::session_source::Parsed<'a>> {
    check(cancel)?;
    let parsed = source
        .parse()
        .map_err(|error| Error::new(E_INVALIDARG, format!("Session decode rejected: {error:?}")))?;
    check(cancel)?;
    Ok(parsed)
}
fn finish_source(
    parsed: crate::session_source::Parsed<'_>,
    placements: Vec<Placement>,
    images: Vec<Images>,
    cancel: &AtomicBool,
) -> Result<Candidate> {
    let document = parsed.document();
    check(cancel)?;
    let projection_scope = crate::session_projection_scope::Proof::capture_parsed(&parsed);
    let account_index = document
        .projection_scope
        .map(|_| {
            crate::projection_account::scene::Index::capture_parsed(
                &parsed,
                projection_scope.as_ref(),
            )
            .map_err(|error| Error::new(E_INVALIDARG, format!("Invalid account scene: {error:?}")))
        })
        .transpose()?;
    let indices = crate::projection_delivery::checkpoint_session::capture(&parsed);
    let original = parsed.source().clone();
    let document = parsed.into_document();
    Ok(Candidate {
        account_index,
        projection_index: indices.receiver,
        sender_index: indices.sender,
        projection_scope,
        origin: None,
        schema_version: document.schema_version,
        revision: document.revision,
        coordinate_space: document.coordinate_space,
        workspace: document.workspace,
        placements,
        images,
        original,
    })
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Session preparation cancelled"))
    } else {
        Ok(())
    }
}
