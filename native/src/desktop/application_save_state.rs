//! 保存后变化凭证：只保留轻量模型身份/版本和几何，不持有像素或整套 Editor 快照。
use super::*;
use crate::{
    editor::DocumentStamp,
    workspace::{Group, TopologyStamp},
    workspace_id::UnitId,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Status {
    Unsaved,
    Unchanged,
    Changed,
}
pub(super) struct Stamp {
    live_snapshot: bool,
    live: Option<save_snapshot::LiveStamp>,
    space: Option<((i32, i32), f64)>,
    topology: TopologyStamp,
    appearance: crate::workspace::ports::AppearanceStamp,
    groups: Vec<Group>,
    membership: Vec<Option<UnitId>>,
    documents: Vec<Document>,
}
struct Document {
    unit: UnitId,
    editor: DocumentStamp,
    output: crate::desktop::loom_result_state::OutputStamp,
    configuration: crate::desktop::configuration_stamp::Stamp,
    projection_edit: Option<crate::desktop::projection_edit_workspace::Stamp>,
    rect: RECT,
    dpi: u32,
    presentation: Presentation,
}
#[derive(PartialEq)]
struct Presentation {
    viewport: crate::viewport::Viewport,
    pinned: bool,
    normal_opacity: f32,
    mini_opacity: f32,
    mini: Option<(crate::session_minified::Views, crate::crop_geometry::Size)>,
    restore: Option<crate::minified_transition::Transition>,
    art: bool,
}
impl Presentation {
    fn capture(state: &State) -> Self {
        let canvas = state.canvas.borrow();
        Self {
            viewport: canvas.viewport,
            pinned: canvas.pinned,
            normal_opacity: canvas.normal_opacity,
            mini_opacity: canvas.minified_opacity,
            mini: canvas.minified.as_ref().map(|value| value.geometry()),
            restore: canvas.minified_restore,
            art: state.is_art(),
        }
    }
}
impl Stamp {
    pub(super) unsafe fn capture(app: &Application) -> Result<Self> {
        let mut documents = Vec::with_capacity(app.documents.len());
        for doc in &app.documents {
            if !doc.is_alive() {
                return Err(Error::new(E_ABORT, "Cannot mark closed workspace saved"));
            }
            let mut rect = RECT::default();
            GetWindowRect(doc.window.0, &mut rect)?;
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0);
            if dpi == 0 {
                return Err(Error::new(E_ABORT, "Cannot capture saved window DPI"));
            }
            documents.push(Document {
                unit: doc.state.unit_id.clone(),
                editor: doc.state.canvas.borrow().editor.document_stamp(),
                output: doc.state.execution_results.borrow().output_stamp(),
                configuration: crate::desktop::configuration_stamp::Stamp::capture(&doc.state),
                projection_edit: crate::desktop::projection_edit_workspace::stamp(
                    doc.state.projection_edit.borrow().as_ref(),
                ),
                rect,
                dpi,
                presentation: Presentation::capture(&doc.state),
            });
        }
        Ok(Self {
            live_snapshot: has_live(app),
            live: None,
            space: space(app),
            topology: app.workspace.topology_stamp(),
            appearance: app.workspace.appearance_stamp(),
            groups: app.workspace.groups().to_vec(),
            membership: app
                .workspace
                .units()
                .iter()
                .map(|unit| app.workspace.group_of(unit).map(|group| group.id.clone()))
                .collect(),
            documents,
        })
    }
    pub(super) unsafe fn capture_snapshot(
        app: &Application,
        receipt: &save_snapshot::Receipt,
    ) -> Result<Self> {
        let mut stamp = Self::capture(app)?;
        // 必须来自真正被 worker 编码的 Receipt，不能把后来的流帧误记为已保存。
        stamp.live = Some(receipt.live_stamp());
        Ok(stamp)
    }
    pub(super) unsafe fn matches(&self, app: &Application) -> bool {
        // 手动 dirty 语义不变：持续 Live 不能被一个时间点快照标记为永久干净。
        !self.live_snapshot && !has_live(app) && self.matches_model(app)
    }
    pub(super) unsafe fn matches_automatic(&self, app: &Application) -> bool {
        self.live_snapshot == has_live(app)
            && (!self.live_snapshot || self.live.as_ref().is_some_and(|live| live.matches(app)))
            && self.matches_model(app)
    }
    pub(super) unsafe fn matches_model(&self, app: &Application) -> bool {
        self.space == space(app)
            && self.topology.matches(&app.workspace)
            && self.appearance.matches(&app.workspace)
            && self.groups == app.workspace.groups()
            && self.documents.len() == app.documents.len()
            && self
                .membership
                .iter()
                .zip(app.workspace.units())
                .all(|(group, unit)| {
                    group.as_ref() == app.workspace.group_of(unit).map(|group| &group.id)
                })
            && self
                .documents
                .iter()
                .zip(&app.documents)
                .all(|(saved, doc)| {
                    let mut rect = RECT::default();
                    saved.unit == doc.state.unit_id
                        && !pending_edits(&doc.state)
                        && doc.is_alive()
                        && GetWindowRect(doc.window.0, &mut rect).is_ok()
                        && saved.rect == rect
                        && saved.dpi == windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0)
                        && saved.editor.matches(&doc.state.canvas.borrow().editor)
                        // 正式端口独立于Editor；预览/进度不推进此身份，也不持有整图。
                        && saved.output.matches(&doc.state.execution_results.borrow())
                        && saved.configuration.matches(&doc.state)
                        && saved.projection_edit == crate::desktop::projection_edit_workspace::stamp(doc.state.projection_edit.borrow().as_ref())
                        && saved.presentation == Presentation::capture(&doc.state)
                })
    }
}
pub(super) fn has_live(app: &Application) -> bool {
    app.live_sessions
        .iter()
        .any(|session| session.belongs_to(&app.workspace))
}
fn pending_edits(state: &State) -> bool {
    let canvas = state.canvas.borrow();
    crate::desktop::group_lock::editing_busy(state)
        || canvas.editor.draft().is_some()
        || canvas.crop_drag.is_some()
        || canvas.crop_pending.is_some()
        || canvas.path_drag.is_some()
        || canvas.rotation_drag.is_some()
        || canvas.scale_drag.is_some()
        || canvas.scale_pending.is_some()
        || canvas.selection_drag.is_some()
        || canvas.selection_effect_preview.is_some()
        || !canvas.group_preview.is_empty()
        || canvas.erase_gesture.is_some()
        || canvas.erase_pending.is_some()
}
impl Application {
    /// 坐标上下文安装完成后才建立普通文件导入的基线；恢复副本仍要求另存为。
    pub(super) unsafe fn record_loaded_save_state(&mut self) {
        self.saved_state = if self
            .restored_session
            .as_ref()
            .is_some_and(|record| record.saved_path.is_some() && record.space.is_some())
        {
            Stamp::capture(self).ok()
        } else {
            None
        };
    }
    /// 按需查询，不引入常驻轮询；这是版本变化判定，不宣称撤销后内容相等即可自动变干净。
    pub(super) unsafe fn save_status(&self) -> Status {
        match &self.saved_state {
            None => Status::Unsaved,
            Some(saved) if saved.matches(self) => Status::Unchanged,
            Some(_) => Status::Changed,
        }
    }
}

fn space(app: &Application) -> Option<((i32, i32), f64)> {
    app.restored_session
        .as_ref()?
        .space
        .map(|value| (value.origin, value.scale))
}

pub(super) unsafe fn verify(app: &mut Application) -> Result<()> {
    if app.save_status() != Status::Unchanged {
        return Err(Error::new(
            E_FAIL,
            "Successful save was not marked unchanged",
        ));
    }
    let original_space = app.restored_session.as_ref().unwrap().space;
    app.restored_session.as_mut().unwrap().space = None;
    let space_changed = app.save_status() == Status::Changed;
    app.restored_session.as_mut().unwrap().space = original_space;
    if !space_changed || app.save_status() != Status::Unchanged {
        return Err(Error::new(
            E_FAIL,
            "Saved coordinate space change was not tracked",
        ));
    }
    let document = &app.documents[0];
    let modal = document.state.file_dialog.replace(true);
    let hotkey = app.hotkey_enabled.get();
    app.hotkey_enabled.set(!hotkey);
    let transient_unchanged = app.save_status() == Status::Unchanged;
    app.hotkey_enabled.set(hotkey);
    document.state.file_dialog.set(modal);
    if !transient_unchanged {
        return Err(Error::new(
            E_FAIL,
            "Non-persistent modal/hotkey state dirtied saved workspace",
        ));
    }
    let opacity = document.state.canvas.borrow().normal_opacity;
    document.state.canvas.borrow_mut().normal_opacity = if opacity == 0.5 { 0.6 } else { 0.5 };
    let changed = app.save_status() == Status::Changed;
    document.state.canvas.borrow_mut().normal_opacity = opacity;
    if !changed || app.save_status() != Status::Unchanged {
        return Err(Error::new(
            E_FAIL,
            "Saved presentation change was not tracked",
        ));
    }
    let began = document
        .state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Line,
            crate::viewport::Point { x: 1.0, y: 1.0 },
        )
        .map_err(|_| Error::new(E_FAIL, "Cannot prepare saved state draft fixture"))?;
    let draft_changed = app.save_status() == Status::Changed;
    document.state.canvas.borrow_mut().editor.cancel();
    if !began || !draft_changed || app.save_status() != Status::Unchanged {
        return Err(Error::new(
            E_FAIL,
            "Uncommitted draft state was not tracked",
        ));
    }
    println!("native-save-state-smoke: committed-unchanged=true presentation-tracked=true transient-ignored=true draft-tracked=true space-tracked=true");
    Ok(())
}
