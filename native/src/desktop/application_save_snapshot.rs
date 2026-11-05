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
    pub pinned: bool,
    pub art: bool,
    pub local_art_id: Option<String>,
    pub local_art_ports: Option<crate::art_execution_grant::package::ports::Record>,
    pub group: Option<UnitId>,
    pub parameters: Option<crate::loom_parameter_state::Parameters>,
    pub(in crate::desktop) sticker: Option<(crate::desktop::sticker_input::Config, bool)>,
    pub execution: Option<crate::graph_execution_config::Config>,
    pub restored_source: Option<crate::desktop::image::Image>,
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
pub struct Receipt {
    projection_scope: crate::session_projection_scope::Scope,
    live: Vec<live::Guard>,
    guard: Guard,
    original: Source,
    space: Space,
}
/// 自动持久化只记已编码时间点的几何/身份，不保 GPU 帧，也不跟踪流序号。
pub(super) struct LiveStamp(Vec<live::Guard>);
impl LiveStamp {
    pub(super) fn matches(&self, app: &Application) -> bool {
        live::matches(&self.0, app)
    }
}
pub(super) fn automatic_ready(app: &Application) -> Result<bool> {
    Ok(app.live_freeze_job.is_none() && live::automatic_ready(app)?)
}
impl Receipt {
    pub(super) fn live_stamp(&self) -> LiveStamp {
        LiveStamp(self.live.clone())
    }
    pub unsafe fn matches(&self, app: &Application) -> bool {
        app.restore_job.is_none()
            && app.restored_session.as_ref().is_some_and(|record| {
                record.original.same_storage(&self.original)
                    && record.projection_scope == Some(self.projection_scope)
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
        let mut documents = Vec::with_capacity(self.documents.len());
        let mut projection_edits = Vec::new();
        for doc in &self.documents {
            let mut rect = RECT::default();
            GetWindowRect(doc.window.0, &mut rect)?;
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
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0);
            if dpi == 0
                || !canvas.viewport.scale.is_finite()
                || canvas.viewport.scale <= 0.0
                || !canvas.viewport.origin.x.is_finite()
                || !canvas.viewport.origin.y.is_finite()
            {
                return Err(Error::new(E_INVALIDARG, "Invalid snapshot viewport or DPI"));
            }
            let selected = doc.state.selected_art.borrow();
            if let Some(owner) = doc.state.projection_edit.borrow().as_ref() {
                let snapshot = owner.capture(&canvas.editor);
                if projection_edits.len() >= crate::projection_edit::workspace::MAX_ENTRIES
                    || !snapshot
                        .belongs_to(doc.state.unit_id.as_str(), record.projection_scope.unwrap())
                {
                    return Err(Error::new(
                        E_ABORT,
                        "Projection edit workspace identity or capacity changed",
                    ));
                }
                projection_edits.push((doc.state.unit_id.clone(), snapshot));
            }
            documents.push(Document {
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
                restored_source: canvas
                    .minified_source
                    .as_ref()
                    .filter(|source| source.matches(&canvas.editor))
                    .map(|source| source.image().clone()),
                local_image: canvas.image.clone(),
            });
        }
        let (live_guard, live_frames) = live::capture(self)?;
        if !guard.matches(self) || !live::matches(&live_guard, self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed during save snapshot",
            ));
        }
        let original = record.original.clone();
        Ok((
            Receipt {
                projection_scope: record.projection_scope.unwrap(),
                live: live_guard,
                guard,
                original: original.clone(),
                space,
            },
            Data {
                projection_edits,
                disk_revision: None,
                projection_scope: record.projection_scope.unwrap(),
                live: live_frames,
                original,
                space,
                documents,
                links: self.workspace.links().to_vec(),
                port_visibility: self.workspace.visibility_records().to_vec(),
                groups: self.workspace.groups().to_vec(),
                transient_host: record.transient_host.clone(),
            },
        ))
    }
}

#[path = "application_save_snapshot_smoke.rs"]
pub(super) mod smoke;
