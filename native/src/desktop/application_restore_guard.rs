//! Source credentials for restored-window batches; do not copy document pixels or substitute catalog identity for workspace-state checks.
use super::*;
#[path = "application_restore_document.rs"]
mod document;
#[path = "application_restore_idle.rs"]
mod idle;
use crate::workspace::{GroupStamp, TopologyStamp};
pub(in crate::desktop::application) use document::Document;
pub(in crate::desktop::application) fn is_idle(app: &Application) -> bool {
    services_idle(app) && app.documents.iter().all(|doc| idle::state(&doc.state))
}
fn services_idle(app: &Application) -> bool {
    idle::services(app) && !app.modals.blocked() && !app.exit_requested.get()
}
pub(in crate::desktop::application) struct Guard {
    hotkey_enabled: bool,
    long_hotkey_enabled: bool,
    live_hotkey_enabled: bool,
    topology: TopologyStamp,
    appearance: crate::workspace::ports::AppearanceStamp,
    groups: GroupStamp,
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
            groups: app.workspace.group_stamp(),
            documents,
        })
    }
    /// Captured physical geometry, in the same document order checked by `matches`.
    pub(in crate::desktop::application) fn geometries(
        &self,
    ) -> impl Iterator<Item = (RECT, u32)> + '_ {
        self.documents.iter().map(Document::geometry)
    }
    fn matches_header(&self, app: &Application) -> bool {
        // Per-window validation includes the same idle::state; do not first repeat this check for the entire batch.
        services_idle(app)
            && self.topology.matches(&app.workspace)
            && self.appearance.matches(&app.workspace)
            && self.hotkey_enabled == app.hotkey_enabled.get()
            && self.long_hotkey_enabled == app.long_hotkey_enabled.get()
            && self.live_hotkey_enabled == app.live_hotkey_enabled.get()
            && self.groups.matches(&app.workspace)
            && self.documents.len() == app.documents.len()
    }
    pub(in crate::desktop::application) unsafe fn matches(&self, app: &Application) -> bool {
        self.matches_header(app)
            && self
                .documents
                .iter()
                .zip(&app.documents)
                .all(|(before, doc)| before.matches(doc))
    }
    pub(in crate::desktop::application) unsafe fn matches_saved(
        &self,
        app: &Application,
        saved: &save_state::Stamp,
    ) -> bool {
        // Compare both baselines independently, but read each HWND's physical observations only once.
        self.matches_header(app)
            && saved.matches_with_restore(app, |index, doc, rect, dpi| {
                self.documents[index].matches_observed(doc, rect, dpi)
            })
    }
}
