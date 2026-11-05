//! 菜单可访问性快照及有界操作队列；句柄不借用 HWND、Panel 或业务模型。
use super::panel::Row;
use std::sync::{Arc, Mutex};
#[path = "native_menu_accessibility_links.rs"]
mod links;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Focus,
    Invoke,
    Expand,
    Collapse,
    ScrollTo(usize),
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Scroll {
    pub first: usize,
    pub visible: usize,
    pub total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub shortcut: String,
    pub enabled: bool,
    pub separator: bool,
    pub submenu: bool,
    pub expanded: bool,
    pub checked: bool,
    pub radio: bool,
    pub focused: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    Unavailable,
    Disabled,
    Unsupported,
    Busy,
}

struct Snapshot {
    active: bool,
    location: Option<(usize, i32)>,
    parent: Option<links::WeakNode>,
    child: Option<(usize, std::sync::Weak<Mutex<Snapshot>>)>,
    identity: u64,
    alive: bool,
    generation: u64,
    entries: Vec<Entry>,
    keys: Vec<usize>,
    scroll: Scroll,
    pending: std::collections::VecDeque<(u64, usize, Action)>,
}
pub(crate) struct Owner(Arc<Mutex<Snapshot>>);
#[derive(Clone)]
pub(crate) struct View(Arc<Mutex<Snapshot>>);
#[derive(Clone)]
pub struct Node {
    snapshot: Arc<Mutex<Snapshot>>,
    generation: u64,
    index: usize,
}

impl Owner {
    pub fn new(rows: &[Row]) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let identity = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if identity == u64::MAX {
            std::process::abort();
        }
        Self(Arc::new(Mutex::new(Snapshot {
            active: true,
            location: None,
            parent: None,
            child: None,
            identity,
            alive: true,
            generation: 1,
            entries: rows.iter().map(entry).collect(),
            keys: rows.iter().map(|row| row.key).collect(),
            scroll: Scroll {
                first: 0,
                visible: rows.len(),
                total: rows.len(),
            },
            pending: Default::default(),
        })))
    }
    pub fn replace(&self, rows: &[Row]) {
        let Ok(mut state) = self.0.lock() else { return };
        let mut entries: Vec<_> = rows.iter().map(entry).collect();
        let keys: Vec<_> = rows.iter().map(|row| row.key).collect();
        // 焦点改变不撤销节点，允许 SetFocus 后对同一节点 Invoke；滚动后的
        // 同名行则由模型索引区别，不能用文案或可见槽位充当身份。
        if keys == state.keys
            && entries.iter().zip(&state.entries).all(|(new, old)| {
                new.name == old.name
                    && new.shortcut == old.shortcut
                    && new.enabled == old.enabled
                    && new.separator == old.separator
                    && new.submenu == old.submenu
                    && new.checked == old.checked
                    && new.radio == old.radio
            })
        {
            for (new, old) in entries.iter_mut().zip(&state.entries) {
                new.expanded = old.expanded;
            }
            state.entries = entries;
            return;
        }
        let Some(generation) = state.generation.checked_add(1) else {
            state.alive = false;
            state.pending.clear();
            return;
        };
        state.generation = generation;
        state.child = None;
        state.entries = entries;
        state.keys = keys;
        state.pending.clear();
    }
    pub fn nodes(&self) -> Vec<Node> {
        self.view().nodes()
    }
    pub fn view(&self) -> View {
        View(self.0.clone())
    }
    pub fn take(&self) -> Option<(usize, Action)> {
        take(&self.0)
    }
    pub fn set_expanded(&self, key: Option<usize>) {
        let Ok(mut state) = self.0.lock() else { return };
        for index in 0..state.entries.len() {
            let expanded = key == Some(state.keys[index]);
            let entry = &mut state.entries[index];
            entry.expanded = entry.submenu && expanded;
        }
    }
    pub fn set_scroll(&self, scroll: Scroll) {
        if let Ok(mut state) = self.0.lock() {
            state.scroll = scroll;
        }
    }
    pub fn set_active(&self, active: bool) {
        if let Ok(mut state) = self.0.lock() {
            state.active = active;
        }
    }
    pub fn revoke(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.alive = false;
            state.location = None;
            state.parent = None;
            state.child = None;
            state.entries.clear();
            state.keys.clear();
            state.pending.clear();
        }
    }
}
impl View {
    pub fn scroll(&self) -> Option<Scroll> {
        let state = self.0.lock().ok()?;
        state.alive.then_some(state.scroll)
    }
    pub fn scroll_to(&self, first: usize) -> Result<(), Rejected> {
        let mut state = self.0.lock().map_err(|_| Rejected::Unavailable)?;
        if !state.alive {
            return Err(Rejected::Unavailable);
        }
        if state.scroll.total <= state.scroll.visible {
            return Err(Rejected::Unsupported);
        }
        if state.pending.len() >= 32 {
            return Err(Rejected::Busy);
        }
        let first = first.min(state.scroll.total - state.scroll.visible);
        let generation = state.generation;
        state
            .pending
            .push_back((generation, usize::MAX, Action::ScrollTo(first)));
        Ok(())
    }
    pub fn alive(&self) -> bool {
        self.0.lock().is_ok_and(|state| state.alive)
    }
    pub fn nodes(&self) -> Vec<Node> {
        let Ok(state) = self.0.lock() else {
            return Vec::new();
        };
        if !state.alive {
            return Vec::new();
        }
        (0..state.entries.len())
            .map(|index| Node {
                snapshot: self.0.clone(),
                generation: state.generation,
                index,
            })
            .collect()
    }
}
fn take(snapshot: &Mutex<Snapshot>) -> Option<(usize, Action)> {
    let mut state = snapshot.lock().ok()?;
    if !state.alive {
        return None;
    }
    while let Some((generation, index, action)) = state.pending.pop_front() {
        if generation == state.generation
            && (matches!(action, Action::ScrollTo(_)) && index == usize::MAX
                || state
                    .entries
                    .get(index)
                    .is_some_and(|entry| allowed(entry, action).is_ok()))
        {
            return Some((index, action));
        }
    }
    None
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.revoke();
    }
}
impl Node {
    pub(crate) fn slot(&self) -> usize {
        self.index
    }
    pub(crate) fn runtime_id(&self) -> Option<[i32; 5]> {
        let state = self.snapshot.lock().ok()?;
        (state.alive && state.generation == self.generation).then_some([
            state.identity as i32,
            (state.identity >> 32) as i32,
            self.generation as i32,
            (self.generation >> 32) as i32,
            self.index as i32,
        ])
    }
    pub fn read(&self) -> Option<Entry> {
        let state = self.snapshot.lock().ok()?;
        if !state.alive || state.generation != self.generation {
            return None;
        }
        let mut entry = state.entries.get(self.index)?.clone();
        // 展开祖先的视觉高亮不是第二个键盘焦点，不改变绘制层的选中状态。
        entry.focused &= state.active;
        Some(entry)
    }
    pub fn enqueue(&self, action: Action) -> Result<(), Rejected> {
        let mut state = self.snapshot.lock().map_err(|_| Rejected::Unavailable)?;
        if !state.alive || state.generation != self.generation {
            return Err(Rejected::Unavailable);
        }
        allowed(
            state.entries.get(self.index).ok_or(Rejected::Unavailable)?,
            action,
        )?;
        if state.pending.len() >= 32 {
            return Err(Rejected::Busy);
        }
        state
            .pending
            .push_back((self.generation, self.index, action));
        Ok(())
    }
}
fn allowed(entry: &Entry, action: Action) -> Result<(), Rejected> {
    if matches!(action, Action::ScrollTo(_)) {
        return Err(Rejected::Unsupported);
    }
    if !entry.enabled || entry.separator {
        return Err(Rejected::Disabled);
    }
    if matches!(action, Action::Invoke) && entry.submenu
        || matches!(action, Action::Expand | Action::Collapse) && !entry.submenu
    {
        return Err(Rejected::Unsupported);
    }
    Ok(())
}
fn entry(row: &Row) -> Entry {
    let (label, shortcut) = row.label.split_once('\t').unwrap_or((&row.label, ""));
    let mut chars = label.chars();
    let mut name = String::new();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            if let Some(next) = chars.next() {
                name.push(next);
            }
        } else {
            name.push(ch);
        }
    }
    Entry {
        name,
        shortcut: shortcut.into(),
        enabled: row.enabled,
        separator: row.separator,
        submenu: row.submenu,
        expanded: false,
        checked: row.checked,
        radio: row.radio,
        focused: row.focused,
    }
}

#[cfg(test)]
#[path = "native_menu_accessibility_tests.rs"]
mod tests;
