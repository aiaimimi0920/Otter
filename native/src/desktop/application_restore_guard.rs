//! 恢复窗口批次的来源凭证；不复制文档像素，不以目录身份替代工作区状态检查。
use super::*;
#[path = "application_restore_document.rs"]
mod document;
#[path = "application_restore_idle.rs"]
mod idle;
use crate::{
    workspace::{Group, TopologyStamp},
    workspace_id::UnitId,
};
pub(in crate::desktop::application) use document::Document;
pub(in crate::desktop::application) fn is_idle(app: &Application) -> bool {
    idle::application(app) && !app.modals.blocked() && !app.exit_requested.get()
}
pub(in crate::desktop::application) struct Guard {
    hotkey_enabled: bool,
    long_hotkey_enabled: bool,
    live_hotkey_enabled: bool,
    topology: TopologyStamp,
    appearance: crate::workspace::ports::AppearanceStamp,
    groups: Vec<Group>,
    membership: Vec<Option<UnitId>>,
    documents: Vec<Document>,
}
impl Guard {
    pub(in crate::desktop::application) unsafe fn capture(app: &Application) -> Result<Self> {
        let mut documents = Vec::with_capacity(app.documents.len());
        for doc in &app.documents {
            documents.push(Document::capture(doc)?);
        }
        Ok(Self {
            hotkey_enabled: app.hotkey_enabled.get(),
            long_hotkey_enabled: app.long_hotkey_enabled.get(),
            live_hotkey_enabled: app.live_hotkey_enabled.get(),
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
    pub(in crate::desktop::application) unsafe fn matches(&self, app: &Application) -> bool {
        if !is_idle(app) {
            return false;
        }
        if !self.topology.matches(&app.workspace)
            || !self.appearance.matches(&app.workspace)
            || self.hotkey_enabled != app.hotkey_enabled.get()
            || self.long_hotkey_enabled != app.long_hotkey_enabled.get()
            || self.live_hotkey_enabled != app.live_hotkey_enabled.get()
            || self.groups != app.workspace.groups()
            || self.documents.len() != app.documents.len()
        {
            return false;
        }
        if !self
            .membership
            .iter()
            .zip(app.workspace.units())
            .all(|(group, unit)| {
                group.as_ref() == app.workspace.group_of(unit).map(|group| &group.id)
            })
        {
            return false;
        }
        self.documents
            .iter()
            .zip(&app.documents)
            .all(|(before, doc)| before.matches(doc))
    }
}
