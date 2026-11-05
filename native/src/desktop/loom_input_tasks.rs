//! 应用拥有输入准备任务，最多两槽；未领取结果也占槽，关闭只取消而不阻塞 UI。
use super::{
    loom_execution_signal::Notify,
    loom_input_job::{self, Job, PreparedBatch, Source},
};
use crate::{editor::Editor, workspace_id::UnitId};
#[derive(Debug)]
pub enum Error {
    Capacity,
    Duplicate,
    Allocation,
    Start(loom_input_job::Error),
}
struct Task {
    ticket: UnitId,
    target: UnitId,
    job: Job,
    retired: bool,
    result: Option<Result<PreparedBatch, loom_input_job::Error>>,
}
pub struct Report {
    pub ticket: UnitId,
    pub target: UnitId,
    pub result: Result<PreparedBatch, loom_input_job::Error>,
}
pub struct Tasks {
    entries: Vec<Task>,
    notify: Notify,
}
impl Tasks {
    pub fn new(notify: Notify) -> Self {
        Self {
            entries: Vec::new(),
            notify,
        }
    }
    pub fn start(
        &mut self,
        ticket: UnitId,
        target: UnitId,
        editor: &Editor,
        sources: Vec<Source>,
    ) -> Result<(), Error> {
        if self.entries.len() >= 2 {
            return Err(Error::Capacity);
        }
        if self
            .entries
            .iter()
            .any(|task| task.ticket == ticket || (task.target == target && !task.retired))
        {
            return Err(Error::Duplicate);
        }
        self.entries.try_reserve(1).map_err(|_| Error::Allocation)?;
        let mut job = Job::default();
        job.start_with_notify(target.clone(), editor, sources, self.notify.clone())
            .map_err(Error::Start)?;
        self.entries.push(Task {
            ticket,
            target,
            job,
            retired: false,
            result: None,
        });
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn retain_units(&mut self, mut exists: impl FnMut(&UnitId) -> bool) {
        self.retain_requests(|unit, _| exists(unit));
    }
    pub fn retain_requests(&mut self, mut keep: impl FnMut(&UnitId, &UnitId) -> bool) {
        for task in &mut self.entries {
            if !keep(&task.target, &task.ticket) {
                task.retired = true;
                task.job.cancel();
            }
        }
    }
    pub fn cancel(&mut self, ticket: &UnitId) {
        for task in &mut self.entries {
            if &task.ticket == ticket {
                task.retired = true;
                task.job.cancel();
            }
        }
    }
    pub fn poll(&mut self) {
        for task in &mut self.entries {
            if task.result.is_none() {
                task.result = task.job.poll();
            }
        }
    }
    /// 只丢弃已经 join 的退休结果；仍在取消/编码中的 owner 保留，不在 UI 上等待。
    pub fn discard_retired(&mut self) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|task| !(task.retired && task.result.is_some()));
        before - self.entries.len()
    }
    /// 领取后仍须核验配置 ticket 和 PreparedBatch 的全部文档凭证。
    pub fn take_completed(&mut self) -> Option<Report> {
        self.take_matching(|_, _| true)
    }
    pub fn take_matching(
        &mut self,
        mut matches: impl FnMut(&UnitId, &UnitId) -> bool,
    ) -> Option<Report> {
        let index = self
            .entries
            .iter()
            .position(|task| task.result.is_some() && matches(&task.target, &task.ticket))?;
        let mut task = self.entries.remove(index);
        let result = task.result.take().unwrap();
        Some(Report {
            ticket: task.ticket,
            target: task.target,
            result: if task.retired {
                Err(loom_input_job::Error::Cancelled)
            } else {
                result
            },
        })
    }
    pub fn shutdown(&mut self) {
        for task in &self.entries {
            task.job.cancel();
        }
        for mut task in self.entries.drain(..) {
            let _ = task.job.shutdown();
        }
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaced_ticket_reclaims_only_old_completed_owner() {
        let _serial = loom_input_job::TEST_SERIAL.lock().unwrap();
        let id = |value| UnitId::parse(value).unwrap();
        let editor = Editor::default();
        let (send, receive) = std::sync::mpsc::channel();
        let mut tasks = Tasks::new(std::sync::Arc::new(move || {
            send.send(()).unwrap();
        }));
        tasks
            .start(id("old"), id("unit"), &editor, Vec::new())
            .unwrap();
        tasks.retain_requests(|_, _| false);
        assert_eq!(tasks.discard_retired(), 0);
        tasks
            .start(id("new"), id("unit"), &editor, Vec::new())
            .unwrap();
        for _ in 0..2 {
            receive
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        tasks.poll();
        tasks.retain_requests(|unit, ticket| *unit == id("unit") && *ticket == id("new"));
        assert_eq!(tasks.discard_retired(), 1);
        assert_eq!(tasks.len(), 1);
        let report = tasks.take_completed().unwrap();
        assert_eq!(report.ticket, id("new"));
        assert!(report.result.is_ok());
        assert!(tasks.is_empty());
        assert_eq!(tasks.discard_retired(), 0);
    }
    #[test]
    fn completed_slots_remain_bounded_and_closed_targets_are_not_delivered() {
        let _serial = loom_input_job::TEST_SERIAL.lock().unwrap();
        let id = |value| UnitId::parse(value).unwrap();
        let editor = Editor::default();
        let (send, receive) = std::sync::mpsc::channel();
        let mut tasks = Tasks::new(std::sync::Arc::new(move || {
            send.send(()).unwrap();
        }));
        tasks
            .start(id("ticket-a"), id("a"), &editor, Vec::new())
            .unwrap();
        assert!(matches!(
            tasks.start(id("duplicate"), id("a"), &editor, Vec::new()),
            Err(Error::Duplicate)
        ));
        tasks
            .start(id("ticket-b"), id("b"), &editor, Vec::new())
            .unwrap();
        for _ in 0..2 {
            receive
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        tasks.poll();
        assert_eq!(tasks.len(), 2);
        assert!(matches!(
            tasks.start(id("ticket-c"), id("c"), &editor, Vec::new()),
            Err(Error::Capacity)
        ));
        tasks.retain_units(|unit| *unit == id("b"));
        let first = tasks.take_completed().unwrap();
        assert_eq!(first.target, id("a"));
        assert!(matches!(
            first.result,
            Err(loom_input_job::Error::Cancelled)
        ));
        let second = tasks.take_completed().unwrap();
        assert_eq!(second.target, id("b"));
        assert!(second.result.is_ok());
        assert!(tasks.is_empty());
        tasks
            .start(id("ticket-c"), id("c"), &editor, Vec::new())
            .unwrap();
        tasks.shutdown();
        assert!(tasks.is_empty());
        assert!(tasks.take_completed().is_none());
    }
}
