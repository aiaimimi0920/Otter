//! UI线程共享历史快照和有界意图；回调不读写磁盘、不创建窗口、不启动线程。
use super::State;
use crate::{
    capture_history::Data, image_decode::Image, stroke_style::Color, workspace_id::UnitId,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
pub(super) type Shared = Rc<RefCell<View>>;
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
    Reuse(String),
    Delete(String),
}
#[derive(Default)]
pub(super) struct View {
    pub data: Data,
    pub revision: u64,
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
        self.data.color(color, now());
        self.changed();
    }
    pub fn capture(&mut self, id: String, image: Image) {
        if self.captures.len() >= 4 {
            self.status = Some("历史记录繁忙，本次截图未加入历史".into());
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
        // 原版按capture unit身份去重；多次替换同unit也更新而不是重复新增。
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
