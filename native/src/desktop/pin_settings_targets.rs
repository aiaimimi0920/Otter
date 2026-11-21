//! 目标草稿只在确认后派发；查询空窗保留呈现，已确认批次使用固定设备 ID。
use super::*;
use std::collections::{BTreeMap, VecDeque};
#[path = "pin_targets_entries.rs"]
mod directory;
#[path = "pin_targets_window.rs"]
mod window;
use directory::entries;
#[cfg(test)]
#[path = "pin_settings_targets_tests.rs"]
pub(in crate::desktop) mod tests;
#[derive(Clone, PartialEq)]
struct Entry {
    id: String,
    name: String,
    checked: bool,
    enabled: bool,
    command: usize,
}
pub(super) struct Popup {
    menu: window::Window,
    kind: u16,
    origin: Option<String>,
    rows: Vec<Entry>,
    draft: BTreeMap<String, bool>,
    queue: VecDeque<(String, bool)>,
    inflight: Option<(String, bool)>,
    pub(super) applying: bool,
    failed: bool,
    target: crate::editor::DocumentStamp,
    mouse_down: bool,
}
fn merge_draft(draft: &mut BTreeMap<String, bool>, rows: &[Entry]) {
    draft.retain(|id, _| rows.iter().any(|row| &row.id == id));
    for row in rows {
        draft.entry(row.id.clone()).or_insert(row.checked);
    }
}
fn select_all(draft: &mut BTreeMap<String, bool>, rows: &[Entry]) {
    for row in rows.iter().filter(|r| r.enabled) {
        draft.insert(row.id.clone(), true);
    }
}
fn changes(draft: &BTreeMap<String, bool>, rows: &[Entry]) -> Vec<(String, bool)> {
    rows.iter()
        .filter_map(|r| {
            draft
                .get(&r.id)
                .filter(|wanted| **wanted != r.checked)
                .map(|wanted| (r.id.clone(), *wanted))
        })
        .collect()
}
impl Popup {
    pub(super) unsafe fn translate(&self, message: &MSG) -> bool {
        self.menu.translate(message)
    }
    pub(super) unsafe fn open(owner: HWND, state: &State, kind: u16, anchor: RECT) -> Result<Self> {
        let rows = entries(state, kind);
        let menu = window::Window::open(owner, &rows, anchor)?;
        if state.authorization.borrow().targets.directory.is_none()
            && auth::targets::request(&state.authorization.borrow()).is_ok()
        {
            auth::execute(owner, state, auth::targets::REFRESH)?;
        }
        let draft = rows.iter().map(|r| (r.id.clone(), r.checked)).collect();
        if rows.is_empty() {
            menu.status(&directory_status(state));
        }
        Ok(Self {
            menu,
            kind,
            rows,
            draft,
            origin: state.authorization.borrow().origin.clone(),
            queue: VecDeque::new(),
            inflight: None,
            applying: false,
            failed: false,
            target: state.canvas.borrow().editor.document_stamp(),
            mouse_down: true,
        })
    }
    fn confirm(&mut self, state: &State) {
        let changed = changes(&self.draft, &entries(state, self.kind));
        let view = state.authorization.borrow();
        let directory = view
            .targets
            .directory
            .as_ref()
            .expect("confirmed current directory");
        for (id, wanted) in changed {
            // 组/用户在确认时展开，后续目录刷新不得扩大已确认的投射范围。
            let targets = if self.kind == DEVICES {
                vec![id]
            } else if self.kind == GROUPS {
                directory
                    .groups()
                    .unwrap_or_default()
                    .iter()
                    .find(|g| g.id() == id)
                    .map(|g| g.target_ids().to_vec())
                    .unwrap_or_default()
            } else {
                directory
                    .users()
                    .iter()
                    .find(|u| u.id() == id)
                    .map(|u| u.target_ids().to_vec())
                    .unwrap_or_default()
            };
            for target in targets {
                self.queue.retain(|(old, _)| old != &target);
                self.queue.push_back((target, wanted));
            }
        }
        self.applying = true;
    }
    pub(super) unsafe fn poll(&mut self, owner: HWND, state: &State) -> Result<bool> {
        match self.advance(owner, state) {
            Ok(open) => Ok(open),
            Err(error) => {
                // 失败保留用户草稿，只有权威绑定成功才能宣告完成。
                self.applying = false;
                self.failed = true;
                self.inflight = None;
                self.queue.clear();
                self.menu.status(&format!("未完成：{error}"));
                let mut display = self.rows.clone();
                for row in &mut display {
                    row.checked = *self.draft.get(&row.id).unwrap_or(&row.checked);
                }
                self.menu.update(&display, true, true, false);
                Ok(!self.menu.closed())
            }
        }
    }
    unsafe fn advance(&mut self, owner: HWND, state: &State) -> Result<bool> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON,
        };
        // 目标选择属于钉图身份；正常编辑和协作回写不能撤销用户已确认的批次。
        if state.authorization.borrow().origin != self.origin
            || !self.target.same_document(&state.canvas.borrow().editor)
        {
            return Ok(false);
        }
        if !self.applying {
            if !super::super::is_open(state)
                || self.menu.closed()
                || GetAncestor(GetForegroundWindow(), GA_ROOTOWNER)
                    != GetAncestor(owner, GA_ROOTOWNER)
            {
                return Ok(false);
            }
            let mut point = POINT::default();
            let _ = GetCursorPos(&mut point);
            let down = GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0;
            let outside = down && !self.mouse_down && !self.menu.contains(point);
            self.mouse_down = down;
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 || outside {
                return Ok(false);
            }
        }
        if !self.applying {
            self.sync_directory(state);
        }
        while let Some(event) = self.menu.event() {
            if self.applying {
                continue;
            }
            match event {
                window::Event::All => select_all(&mut self.draft, &self.rows),
                window::Event::Toggle(id, checked) => {
                    if self.draft.contains_key(&id) {
                        self.draft.insert(id, checked);
                    }
                }
                window::Event::Refresh => {
                    if auth::targets::request(&state.authorization.borrow()).is_ok() {
                        auth::execute(owner, state, auth::targets::REFRESH)?;
                        self.failed = false;
                        self.menu.status("");
                    }
                }
                window::Event::Confirm => {
                    if state.authorization.borrow().targets.directory.is_some() {
                        self.failed = false;
                        self.confirm(state);
                        self.menu.status("正在应用…");
                    }
                }
            }
        }
        let (current, idle) = {
            let v = state.authorization.borrow();
            if self.applying
                && (v.closed || !v.connected || v.sender.retry.as_ref() == Some(&state.unit_id))
            {
                return Err(windows::core::Error::new(
                    E_ABORT,
                    "投射未完成，请检查连接或重试原请求",
                ));
            }
            (
                v.targets.directory.is_some(),
                v.sender.ready
                    && !state.modals.blocked()
                    && !state.exit_requested.get()
                    && !v.sender.busy
                    && !v.editing.busy
                    && !v.account.busy
                    && !v.targets.busy
                    && !v.projection.busy
                    && !v.busy
                    && v.request.is_none(),
            )
        };
        if self.applying && idle && current {
            let choices = entries(state, DEVICES);
            if let Some((id, wanted)) = self.inflight.take() {
                if !choices
                    .iter()
                    .any(|row| row.id == id && row.checked == wanted)
                {
                    return Err(windows::core::Error::new(
                        E_FAIL,
                        state.authorization.borrow().sender.status.as_str(),
                    ));
                }
            }
            if let Some((id, wanted, command)) =
                next_choice(&mut self.queue, &choices, |c| dispatch_ready(state, c))
            {
                auth::sender::execute(owner, state, command)?;
                if state.authorization.borrow().request.is_none() {
                    return Err(windows::core::Error::new(E_ABORT, "投射请求未排入"));
                }
                self.inflight = Some((id, wanted));
            } else if !self.queue.is_empty() {
                return Err(windows::core::Error::new(
                    E_ABORT,
                    "目标当前不可用，请刷新后重试",
                ));
            }
            if self.queue.is_empty() && self.inflight.is_none() {
                return Ok(false);
            }
        }
        let mut display = self.rows.clone();
        for row in &mut display {
            row.checked = *self.draft.get(&row.id).unwrap_or(&row.checked);
        }
        if self.applying && idle && !current {
            return Err(windows::core::Error::new(
                E_ABORT,
                "目标目录不可用，请刷新后重试",
            ));
        }
        self.menu
            .update(&display, current, self.origin.is_some(), self.applying);
        Ok(true)
    }
    unsafe fn sync_directory(&mut self, state: &State) {
        let was_empty = self.rows.is_empty();
        if state.authorization.borrow().targets.directory.is_some() {
            self.rows = entries(state, self.kind);
            merge_draft(&mut self.draft, &self.rows);
        }
        // 投射失败由显式刷新/重试清除，后台空目录提示不能覆盖失败原因。
        if !self.failed {
            if self.rows.is_empty() {
                self.menu.status(&directory_status(state));
            } else if was_empty {
                self.menu.status("");
            }
        }
    }
}
fn directory_status(state: &State) -> String {
    let view = state.authorization.borrow();
    if view.targets.busy || matches!(view.request, Some(auth::Request::ProjectionTargets)) {
        "正在加载…".into()
    } else if view.targets.directory.is_some() {
        "暂无目标".into()
    } else if !view.targets.status.is_empty() {
        view.targets.status.clone()
    } else {
        "目录不可用，请刷新".into()
    }
}
fn next_choice(
    queue: &mut VecDeque<(String, bool)>,
    rows: &[Entry],
    ready: impl Fn(usize) -> bool,
) -> Option<(String, bool, usize)> {
    // 只遍历本轮已有意图，暂不可操作的一项不能堵住其余目标。
    for _ in 0..queue.len() {
        let (id, wanted) = queue.pop_front()?;
        let Some(row) = rows.iter().find(|row| row.id == id) else {
            queue.push_back((id, wanted));
            continue;
        };
        if row.checked == wanted {
            continue;
        }
        if ready(row.command) {
            return Some((id, wanted, row.command));
        }
        queue.push_back((id, wanted));
    }
    None
}

fn dispatch_ready(state: &State, command: usize) -> bool {
    let view = state.authorization.borrow();
    if let Some(index) = command
        .checked_sub(auth::sender::groups::STOP_BASE)
        .filter(|i| *i < crate::projection_delivery::targets::MAX_GROUPS)
    {
        auth::sender::groups::stop_enabled(&view, &state.unit_id, index)
    } else if let Some(index) = command
        .checked_sub(auth::sender::groups::BASE)
        .filter(|i| *i < crate::projection_delivery::targets::MAX_GROUPS)
    {
        auth::sender::groups::enabled(&view, &state.unit_id, index)
    } else if let Some(index) = command
        .checked_sub(auth::sender::STOP_BASE)
        .filter(|i| *i < crate::projection_delivery::sender::binding::MAX_PER_UNIT)
    {
        auth::sender::stops(&view, &state.unit_id)
            .nth(index)
            .is_some_and(|binding| auth::sender::stop_enabled(&view, binding))
    } else {
        command
            .checked_sub(auth::sender::BASE)
            .is_some_and(|index| auth::sender::enabled(&view, &state.unit_id, index))
    }
}
