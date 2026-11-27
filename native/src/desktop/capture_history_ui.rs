//! UI-thread shared history snapshots and bounded intents; callbacks do no disk I/O, window creation, or thread startup.
use super::State;
use crate::{
    capture_history::Data, image_decode::Image, stroke_style::Color, workspace_id::UnitId,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    sync::Arc,
};
pub(super) type Shared = Rc<RefCell<View>>;
#[path = "capture_history_previews.rs"]
pub(super) mod previews;
pub(super) struct Capture {
    pub id: String,
    pub image: Image,
    pub at: u64,
}
#[derive(Clone)]
pub(super) struct Origin {
    pub unit: UnitId,
    pub lifetime: Rc<Cell<bool>>,
}
#[derive(Clone)]
pub(super) enum Action {
    Color(Color, String),
    Reuse(Arc<()>, Image),
    Delete(Arc<()>),
}
#[derive(Default)]
pub(super) struct View {
    pub last_region: Option<crate::capture_last_region::Last>,
    // Window actions and the save worker keep the exact revision they observed.
    pub data: Arc<Data>,
    pub revision: u64,
    pub previews: previews::Previews,
    pub toggle: Option<Origin>,
    pub action: Option<(Origin, Action)>,
    pub captures: VecDeque<Capture>,
    pub status: Option<String>,
}
impl View {
    pub fn changed(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
    pub fn color(&mut self, color: Color) {
        Arc::make_mut(&mut self.data).color(color, now());
        self.changed();
    }
    pub fn screenshot(&mut self, mut entry: crate::capture_history::Screenshot) {
        self.previews.captured(&mut entry);
        Arc::make_mut(&mut self.data).screenshot(entry);
        self.previews.retain_current(&self.data);
        self.changed();
    }
    pub fn remove_screenshot(&mut self, id: &str) {
        if Arc::make_mut(&mut self.data).remove(id) {
            self.previews.retain_current(&self.data);
            self.changed();
        }
    }
    pub fn capture(&mut self, id: String, image: Image) {
        if self.captures.len() >= 4 {
            self.status = Some(crate::ui_text::text("capture-history-busy").into());
            return;
        }
        self.captures.push_back(Capture {
            id,
            image,
            at: now(),
        });
    }
}
pub(super) fn origin(state: &State) -> Origin {
    Origin {
        unit: state.unit_id.clone(),
        lifetime: state.file_dialog.clone(),
    }
}
pub(super) fn record(state: &State) {
    let image = {
        let canvas = state.canvas.borrow();
        canvas.capture_source().and_then(|_| canvas.image.clone())
    };
    if let Some(image) = image {
        // The original implementation deduplicates by capture unit identity; replacing the same unit repeatedly updates rather than appends.
        state
            .capture_history
            .borrow_mut()
            .capture(state.unit_id.to_string(), image);
    }
}
pub(super) fn record_initial(state: &State, capture: bool) {
    if capture {
        if let Some(image) = state.canvas.borrow().image.clone() {
            state
                .capture_history
                .borrow_mut()
                .capture(state.unit_id.to_string(), image);
        }
    }
}
pub(super) fn toggle(state: &State) {
    if !state.modals.blocked() && !state.session_end.paused() {
        state.capture_history.borrow_mut().toggle = Some(origin(state));
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        })
}
