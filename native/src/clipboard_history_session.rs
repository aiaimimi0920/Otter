//! 打开的历史菜单会话：事件合并、异步刷新与可选择快照的唯一 owner。
use super::{events::Events, task::Task, Entry, Snapshot, Status};
use std::time::{Duration, Instant};
use windows::core::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Loading,
    Ready,
    Empty,
    Disabled,
    Denied,
    Unavailable,
}

pub struct Session {
    events: Events,
    task: Option<Task>,
    snapshot: Option<Snapshot>,
    state: State,
    pending: bool,
    next_start: Instant,
    started: Instant,
}

impl Session {
    #[cfg(test)]
    pub(crate) fn test_snapshot(snapshot: Snapshot) -> Self {
        let events = Events::synthetic();
        events.take_changed();
        let mut session = Self::new(events, Instant::now());
        session.pending = false;
        session.accept(Ok(snapshot));
        session
    }

    #[cfg(test)]
    pub(crate) fn test_invalidate(&self) {
        self.events.signal();
    }

    /// 由菜单打开手势调用；订阅失败交给 UI 显示，不偷偷改成轮询系统历史。
    pub fn open() -> Result<Self> {
        Ok(Self::new(Events::subscribe()?, Instant::now()))
    }

    fn new(events: Events, now: Instant) -> Self {
        Self {
            events,
            task: None,
            snapshot: None,
            state: State::Loading,
            pending: true,
            next_start: now,
            started: now,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn entries(&self) -> &[Entry] {
        self.snapshot
            .as_ref()
            .map_or(&[], |snapshot| &snapshot.entries)
    }

    /// UI 定时器只查询本地通道/原子标记；真正读取仅在有待处理变化时启动。
    pub fn tick(&mut self) -> bool {
        self.tick_with(Instant::now(), Task::start)
    }

    fn tick_with(&mut self, now: Instant, start: impl FnOnce() -> Result<Task>) -> bool {
        let mut changed = false;
        if self.events.take_changed() {
            self.pending = true;
            self.snapshot = None;
            self.state = State::Loading;
            changed = true;
        }
        if let Some(task) = self.task.as_mut() {
            if let Some(result) = task.poll() {
                self.task = None;
                // 读取期间发生变化：该快照不再可选，等待下一次读取。
                if !self.pending {
                    self.accept(result);
                    changed = true;
                }
            } else if now.saturating_duration_since(self.started) >= Duration::from_secs(6) {
                // UI 超时只关闭通道，不等待提供方；后台许可仍保护其资源生命周期。
                self.task = None;
                self.state = State::Unavailable;
                changed = true;
            }
        }
        if self.task.is_none() && self.pending && now >= self.next_start {
            self.next_start = now + Duration::from_millis(250);
            match start() {
                Ok(task) => {
                    self.task = Some(task);
                    self.pending = false;
                    self.started = now;
                    self.state = State::Loading;
                }
                Err(_) => self.state = State::Unavailable,
            }
            changed = true;
        }
        changed
    }

    fn accept(&mut self, result: Result<Snapshot>) {
        match result {
            Ok(snapshot) => {
                self.state = match snapshot.status {
                    Status::Ready if snapshot.entries.is_empty() => State::Empty,
                    Status::Ready => State::Ready,
                    Status::Disabled => State::Disabled,
                    Status::Denied => State::Denied,
                };
                self.snapshot = Some(snapshot);
            }
            Err(_) => {
                self.snapshot = None;
                self.state = State::Unavailable;
            }
        }
    }

    /// 点击前再次消费事件，避免最后一次绘制之后的变化使用旧命中索引。
    /// 调用者随后仍须检查窗口/文档目标，并关闭菜单后再交付内容。
    pub fn take_entry(&mut self, index: usize, painted_id: &str) -> Option<Entry> {
        if self.events.take_changed() {
            self.pending = true;
            self.snapshot = None;
            self.state = State::Loading;
        }
        if self.pending || self.task.is_some() || self.state != State::Ready {
            return None;
        }
        let snapshot = self.snapshot.as_mut()?;
        if snapshot.entries.get(index)?.id != painted_id {
            return None;
        }
        let entry = snapshot.entries.remove(index);
        self.snapshot = None;
        self.state = State::Empty;
        Some(entry)
    }
}

#[cfg(test)]
#[path = "clipboard_history_session_tests.rs"]
mod tests;
