//! 参数窗只投影共享菜单快照和端口目录；不另存参数，不复制文件 Data URL，不解析执行图。
use super::*;
pub(super) const PAGE_SIZE: usize = 16;
pub(super) struct Row {
    pub id: String,
    pub label: String,
    pub widget: String,
    pub linked: bool,
    pub primary: Vec<usize>,
    pub reset: Option<usize>,
    pub disable: Option<usize>,
}
pub(super) struct Model {
    pub stamp: Stamp,
    pub snapshot: Option<Rc<Snapshot>>,
    pub rows: Vec<Row>,
    pub ports: Option<Rc<desktop::link_catalog::Catalog>>,
    pub title: String,
    pub settings: Vec<(String, bool)>,
    pub sticker: Option<desktop::sticker_input::Config>,
    pub candidates: Option<std::sync::Arc<desktop::loom_output_batch::candidates::Set>>,
}
impl Model {
    pub fn capture(state: &State) -> Self {
        let snapshot = Snapshot::capture(state).map(Rc::new);
        let mut rows = Vec::new();
        let selected = state.selected_art.borrow();
        let capability = selected.as_ref().and_then(|selected| {
            selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(crate::loom_capability::Capability::new)
                .find(|cap| cap.id == selected.parameters.art_id())
        });
        if let (Some(cap), Some(snapshot)) = (&capability, &snapshot) {
            for parameter in cap.parameters().filter(|p| !p.secret) {
                let mut row = Row {
                    id: parameter.id.into(),
                    label: clean(parameter.label),
                    widget: parameter.widget.clone(),
                    linked: state.parameter_view.borrow().as_ref().is_some_and(|view| {
                        view.current(state) && view.linked.iter().any(|id| id == parameter.id)
                    }),
                    primary: Vec::new(),
                    reset: None,
                    disable: None,
                };
                for (index, choice) in snapshot
                    .choices
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.id == parameter.id)
                {
                    match choice.action {
                        Action::ToggleDisabled(_) => row.disable = Some(index),
                        Action::Reset(_) => row.reset = Some(index),
                        _ => row.primary.push(index),
                    }
                }
                if !row.primary.is_empty() || row.disable.is_some() {
                    rows.push(row);
                }
            }
        }
        let sticker = (!state.is_art()).then(|| state.sticker_input.snapshot());
        if sticker.is_some() {
            rows.push(Row {
                id: "image".into(),
                label: "图片".into(),
                widget: "sticker-image".into(),
                linked: false,
                primary: Vec::new(),
                reset: None,
                disable: None,
            });
        }
        let settings = selected.as_ref().map_or_else(
            || {
                if state.is_art() {
                    Vec::new()
                } else {
                    vec![("接收上游图片".into(), state.accept_upstream.get())]
                }
            },
            |selected| {
                let config = selected.execution;
                [
                    "上游变化触发执行",
                    "参数变化触发执行",
                    "接收上游执行通知",
                    "完成后通知下游",
                ]
                .into_iter()
                .zip([
                    config.upstream_driven,
                    config.param_driven,
                    config.listen_upstream,
                    config.notify_downstream,
                ])
                .map(|(label, checked)| (label.into(), checked))
                .collect()
            },
        );
        let stamp = Stamp::capture(state);
        let candidates = state
            .execution_results
            .borrow()
            .candidate_view(&stamp)
            .cloned();
        Self {
            stamp,
            candidates,
            snapshot,
            rows,
            sticker,
            settings,
            ports: state.link_catalog.borrow().clone(),
            title: capability.map_or_else(
                || {
                    if state.is_art() {
                        "Art".into()
                    } else {
                        "钉图".into()
                    }
                },
                |cap| clean(cap.label),
            ),
        }
    }
    pub fn current(&self, state: &State) -> bool {
        self.stamp.matches(state)
            && match (
                &self.candidates,
                state.execution_results.borrow().candidate_view(&self.stamp),
            ) {
                (None, None) => true,
                (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
                _ => false,
            }
            && self
                .snapshot
                .as_ref()
                .is_none_or(|snapshot| snapshot.current(state))
            && match (&self.ports, state.link_catalog.borrow().as_ref()) {
                (None, None) => true,
                (Some(old), Some(new)) => Rc::ptr_eq(old, new),
                _ => false,
            }
    }
    pub fn pages(&self) -> usize {
        self.rows.len().div_ceil(PAGE_SIZE).max(1)
    }
}
pub(super) fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(128)
        .collect()
}
