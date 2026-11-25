//! 保存快照：UI 线程冻结已提交模型，后台仅消费无 HWND/Rc 的数据；提交前复核凭证。
use super::restore_batch::guard::Guard;
#[path = "application_save_assemble.rs"]
mod assemble;
#[path = "application_save_images.rs"]
pub mod images;
#[path = "application_save_job.rs"]
pub(super) mod job;
#[path = "application_save_live.rs"]
mod live;
#[path = "application_save_vectors.rs"]
mod vectors;
use super::*;
use crate::session_source::Source;
use crate::{
    editor::Editor, pin_geometry::PinRect, session_placement::Space, workspace_id::UnitId,
};

pub struct Document {
    pub node_settings: Option<crate::unit_node_settings::Settings>,
    pub(in crate::desktop) extensions: Option<crate::desktop::extension_document::Snapshot>,
    pub output_images: Vec<(String, crate::desktop::loom_image_value::ImageValue)>,
    pub outputs: Option<serde_json::Map<String, serde_json::Value>>,
    pub capture_original: Option<std::sync::Arc<std::fs::File>>,
    pub capture_meta: Option<serde_json::Value>,
    pub unit: UnitId,
    pub editor: Editor,
    pub frame: PinRect,
    pub viewport: crate::viewport::Viewport,
    pub dpi: u32,
    pub minified: Option<(crate::session_minified::Views, crate::crop_geometry::Size)>,
    pub transition: Option<crate::minified_transition::Transition>,
    pub normal_opacity: f32,
    pub mini_opacity: f32,
    pub frame_style: Option<crate::pin_frame_style::Style>,
    pub pinned: bool,
    pub art: bool,
    pub local_art_id: Option<String>,
    pub local_art_ports: Option<crate::art_execution_grant::package::ports::Record>,
    pub group: Option<UnitId>,
    pub parameters: Option<crate::loom_parameter_state::Parameters>,
    pub(in crate::desktop) sticker: Option<(crate::desktop::sticker_input::Config, bool)>,
    pub execution: Option<crate::graph_execution_config::Config>,
    pub restored_source: Option<crate::desktop::image::Image>,
    pub restored_source_rasterized_crop: bool,
    pub local_image: Option<crate::desktop::image::Image>,
}
impl Document {
    /// 与双击展开共用位移算法，显式区分历史无 savedRect 与主动缩略。
    pub fn geometry(&self) -> Result<crate::session_window_write::Geometry> {
        let expanded = match (self.minified.is_some(), self.transition) {
            (false, None) => None,
            (true, Some(transition)) => transition
                .saved_target((self.frame.x, self.frame.y))
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid save expansion target"))?,
            _ => {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Inconsistent save minified transition",
                ))
            }
        };
        Ok(crate::session_window_write::Geometry {
            frame: self.frame,
            minified: self.minified.is_some(),
            expanded,
            normal_opacity: self.normal_opacity,
            mini_opacity: self.mini_opacity,
        })
    }
}
pub struct Data {
    pub(super) disk_revision: Option<u64>,
    pub(super) projection_scope: crate::session_projection_scope::Scope,
    pub(super) projection_edits: Vec<(UnitId, crate::projection_edit::workspace::Snapshot)>,
    live: Vec<live::Snapshot>,
    pub original: Source,
    pub space: Space,
    pub documents: Vec<Document>,
    pub links: Vec<crate::workspace::Link>,
    pub port_visibility: Vec<(UnitId, crate::workspace::ports::Visibility)>,
    pub groups: Vec<crate::workspace::Group>,
    pub transient_host: Option<UnitId>,
}
/// A library update can encode its requested sticker, never a session candidate.
pub(super) struct FrozenData(Data);
#[derive(Clone, Copy)]
enum Payload<'a> {
    None,
    All,
    Target(&'a UnitId),
}
impl Payload<'_> {
    fn includes(self, unit: &UnitId) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Target(target) => target == unit,
        }
    }
}
pub struct Receipt {
    projection_scope: crate::session_projection_scope::Scope,
    live: Vec<live::Guard>,
    guard: Guard,
    original: Source,
    space: Space,
    transient_host: Option<UnitId>,
}
/// 自动持久化只记已编码时间点的几何/身份，不保 GPU 帧，也不跟踪流序号。
pub(super) struct LiveStamp(Vec<live::Guard>);
impl LiveStamp {
    pub(super) fn matches(&self, app: &Application) -> bool {
        live::matches(&self.0, app)
    }
}
pub(super) fn automatic_static_ready(app: &Application) -> bool {
    // 后台快照只取交互结束状态，不在滚轮/系统移动热路径冻结并序列化整个工作区。
    app.live_freeze_job.is_none()
        && !app
            .documents
            .iter()
            .any(|doc| crate::desktop::pin_window::animation::busy(&doc.state))
}
pub(super) fn automatic_ready(app: &Application) -> Result<bool> {
    Ok(automatic_static_ready(app) && live::automatic_ready(app)?)
}
#[cfg(test)]
pub(super) fn live_frame_reads_for_test() -> usize {
    live::FRAME_READS.with(std::cell::Cell::get)
}
#[cfg(test)]
std::thread_local! {
    static PAYLOAD_DOCUMENTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn payload_documents_for_test() -> usize {
    PAYLOAD_DOCUMENTS.with(std::cell::Cell::get)
}
impl Receipt {
    pub(super) fn original(&self) -> &Source {
        &self.original
    }
    pub(super) unsafe fn checked_saved_stamp(
        &self,
        app: &Application,
    ) -> Option<save_state::Stamp> {
        // Stamp the geometry the worker encoded, not a later window position.
        self.matches(app).then(|| {
            save_state::Stamp::from_verified_geometry(
                app,
                self.guard.geometries(),
                LiveStamp(self.live.clone()),
            )
        })
    }
    pub unsafe fn matches(&self, app: &Application) -> bool {
        app.restore_job.is_none()
            && app.restored_session.as_ref().is_some_and(|record| {
                record.original.same_storage(&self.original)
                    && record.projection_scope == Some(self.projection_scope)
                    && record.transient_host == self.transient_host
                    && record.space.is_some_and(|space| {
                        space.origin == self.space.origin && space.scale == self.space.scale
                    })
            })
            && self.guard.matches(app)
            && live::matches(&self.live, app)
    }
}
impl Application {
    /// 捕获使用创建或恢复时建立的显式坐标上下文；不猜测历史 DPR，不泵消息。
    pub unsafe fn capture_session_snapshot(&mut self) -> Result<(Receipt, Data)> {
        let (receipt, data) = self.build_session_capture(None, Payload::All)?;
        Ok((receipt, data.expect("save must contain payload")))
    }
    pub(super) unsafe fn capture_frozen_snapshot(
        &mut self,
        target: &UnitId,
    ) -> Result<(Receipt, FrozenData)> {
        let (receipt, data) = self.build_session_capture(None, Payload::Target(target))?;
        Ok((
            receipt,
            FrozenData(data.expect("frozen capture must contain payload")),
        ))
    }
    /// 编辑冻结库原文不需要生成新的图像或工作区文档，但必须持有同样的发布凭证。
    pub(super) unsafe fn capture_session_receipt(&mut self) -> Result<Receipt> {
        self.build_session_capture(None, Payload::None)
            .map(|(receipt, _)| receipt)
    }
    pub(super) unsafe fn capture_automatic_session_snapshot(
        &mut self,
    ) -> Result<Option<(Receipt, Data)>> {
        // 调用方已检查静态窗口。同次 UI 操作内移交准入用的帧，不跨消息缓存或领取第二次投递。
        let Some(live) = live::capture(self, true, Payload::All)? else {
            return Ok(None);
        };
        let (receipt, data) = self.build_session_capture(Some(live), Payload::All)?;
        Ok(Some((
            receipt,
            data.expect("automatic save must contain payload"),
        )))
    }
    unsafe fn build_session_capture(
        &mut self,
        live: Option<live::Capture>,
        selection: Payload<'_>,
    ) -> Result<(Receipt, Option<Data>)> {
        for document in &self.documents {
            crate::desktop::extension_document::observe(&document.state);
        }
        // 仅生成固定 32 字节随机身份，不解析原文或访问磁盘；失败不降级为路径/UnitId。
        if let Some(record) = &mut self.restored_session {
            if record.projection_scope.is_none() {
                record.projection_scope = Some(crate::session_projection_scope::Scope::generate()?);
            }
        }
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "No retained session coordinate context"))?;
        let space = record
            .space
            .ok_or_else(|| Error::new(E_ABORT, "Missing session coordinate context"))?;
        let guard = Guard::capture(self)?;
        if self.restore_job.is_some()
            || !guard.matches(self)
            || self.documents.len() > crate::workspace::MAX_UNITS
        {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during save snapshot",
            ));
        }
        let scope = record.projection_scope.unwrap();
        let capacity = match selection {
            Payload::All => self.documents.len(),
            Payload::None => 0,
            Payload::Target(_) => 1,
        };
        let mut documents = Vec::with_capacity(capacity);
        let mut projection_edits = Vec::new();
        let mut transient_host = record.transient_host.clone();
        let mut projection_count = 0;
        // Use the same physical geometry for the payload and its receipt. The
        // checks above and after Live capture still query the actual windows.
        for (doc, (rect, dpi)) in self.documents.iter().zip(guard.geometries()) {
            let frame = PinRect {
                x: rect.left,
                y: rect.top,
                width: u32::try_from(i64::from(rect.right) - i64::from(rect.left))
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid snapshot width"))?,
                height: u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid snapshot height"))?,
            };
            space
                .logical(frame, None)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid snapshot geometry"))?;
            let canvas = doc.state.canvas.borrow();
            if !canvas.viewport.scale.is_finite()
                || canvas.viewport.scale <= 0.0
                || !canvas.viewport.origin.x.is_finite()
                || !canvas.viewport.origin.y.is_finite()
            {
                return Err(Error::new(E_INVALIDARG, "Invalid snapshot viewport"));
            }
            let projection_edit = doc.state.projection_edit.borrow();
            if let Some(owner) = projection_edit.as_ref() {
                if projection_count >= crate::projection_edit::workspace::MAX_ENTRIES
                    || !owner.belongs_to(doc.state.unit_id.as_str(), scope)
                {
                    return Err(Error::new(
                        E_ABORT,
                        "Projection edit workspace identity or capacity changed",
                    ));
                }
                projection_count += 1;
            }
            if !selection.includes(&doc.state.unit_id) {
                continue;
            }
            if let Some(owner) = projection_edit.as_ref() {
                if matches!(selection, Payload::All) {
                    projection_edits
                        .push((doc.state.unit_id.clone(), owner.capture(&canvas.editor)));
                } else if transient_host.as_ref() == Some(&doc.state.unit_id) {
                    // An edit owner promotes the host, but its collaboration payload does not freeze.
                    transient_host = None;
                }
            }
            let selected = doc.state.selected_art.borrow();
            let restored_source = canvas
                .minified_source
                .as_ref()
                .filter(|source| source.matches(&canvas.editor));
            #[cfg(test)]
            PAYLOAD_DOCUMENTS.with(|count| count.set(count.get() + 1));
            documents.push(Document {
                extensions: Some(doc.state.extension_document.borrow().snapshot()),
                output_images: doc.state.execution_results.borrow().save_output_images(),
                outputs: doc.state.execution_results.borrow().save_value_outputs(),
                capture_original: canvas
                    .capture_source()
                    .and_then(|source| source.original_lease.clone()),
                capture_meta: canvas.capture_source().map(|source| source.metadata()),
                unit: doc.state.unit_id.clone(),
                editor: canvas.editor.export_snapshot(),
                frame,
                viewport: canvas.viewport,
                dpi,
                minified: canvas.minified.as_ref().map(|value| value.geometry()),
                transition: canvas.minified_restore,
                normal_opacity: canvas.normal_opacity,
                mini_opacity: canvas.minified_opacity,
                frame_style: Some(canvas.frame_style),
                node_settings: Some(doc.state.node_settings.get()),
                pinned: canvas.pinned,
                art: doc.state.is_art(),
                local_art_id: doc.state.local_art_id.as_deref().map(str::to_owned),
                local_art_ports: doc
                    .state
                    .local_art_id
                    .as_ref()
                    .and_then(|c| c.ports.borrow().as_deref().cloned()),
                group: doc.state.group_id.borrow().clone(),
                parameters: selected.as_ref().map(|value| value.parameters.clone()),
                sticker: (!doc.state.is_art()).then(|| {
                    (
                        doc.state.sticker_input.snapshot(),
                        doc.state.accept_upstream.get(),
                    )
                }),
                execution: selected.as_ref().map(|value| value.execution),
                restored_source_rasterized_crop: restored_source
                    .is_some_and(|source| source.rasterized_crop()),
                restored_source: restored_source.map(|source| source.image().clone()),
                local_image: canvas.image.clone(),
            });
        }
        let live = match live {
            Some(capture) => capture,
            None => {
                live::capture(self, false, selection)?.expect("manual Live capture cannot defer")
            }
        };
        if !guard.matches(self) || !live::matches(&live.guards, self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed during save snapshot",
            ));
        }
        let original = record.original.clone();
        Ok((
            Receipt {
                projection_scope: scope,
                live: live.guards,
                guard,
                original: original.clone(),
                space,
                transient_host: record.transient_host.clone(),
            },
            (!matches!(selection, Payload::None)).then(|| Data {
                projection_edits,
                disk_revision: None,
                projection_scope: scope,
                live: live.frames,
                original,
                space,
                documents,
                // For a frozen sticker, retain only the topology needed to detect an empty host.
                links: self
                    .workspace
                    .links()
                    .iter()
                    .filter(|link| {
                        selection.includes(&link.from_unit) || selection.includes(&link.to_unit)
                    })
                    .cloned()
                    .collect(),
                port_visibility: self
                    .workspace
                    .visibility_records()
                    .iter()
                    .filter(|(unit, _)| selection.includes(unit))
                    .cloned()
                    .collect(),
                groups: if matches!(selection, Payload::All) {
                    self.workspace.groups().to_vec()
                } else {
                    Vec::new()
                },
                transient_host,
            }),
        ))
    }
}

#[path = "application_save_snapshot_smoke.rs"]
pub(super) mod smoke;
