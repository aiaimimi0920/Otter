//! 最多 64 个已接受关联的最旧到期优先调度；单任务复用原 receive owner/全局四槽。
use super::*;
use std::time::{Duration, Instant};
#[derive(Default)]
pub(in crate::desktop::application) struct Schedule {
    pub(super) entries: Vec<Entry>,
    pub(in crate::desktop::application) wake: Option<Instant>,
}
#[cfg(test)]
#[path = "application_projection_sync_schedule_tests.rs"]
mod tests;
pub(super) struct Entry {
    pub(super) ticket: Arc<Ticket>,
    pub(super) binding: Binding,
    pub(super) due: Instant,
    failures: u8,
}
impl Schedule {
    pub(in crate::desktop::application) fn defer_foreground(&mut self, now: Instant) {
        // 显式操作结算后重新等待正常间隔，不在同一轮抢跑；不得缩短原失败退避。
        let resume = now + Duration::from_millis(750);
        for entry in &mut self.entries {
            entry.due = entry.due.max(resume);
        }
        self.wake = None;
    }
    pub(super) fn reconcile(&mut self, mut candidates: Vec<(Arc<Ticket>, Binding)>, now: Instant) {
        candidates.retain(|(_, binding)| !binding.stopped());
        candidates.truncate(64);
        self.entries.retain(|entry| {
            candidates.iter().any(|(ticket, binding)| {
                ticket.same_record(&entry.ticket) && binding == &entry.binding
            })
        });
        for (ticket, binding) in candidates.into_iter().take(64) {
            if !self
                .entries
                .iter()
                .any(|entry| entry.ticket.same_record(&ticket))
            {
                self.entries.push(Entry {
                    ticket,
                    binding,
                    due: now + Duration::from_millis(750),
                    failures: 0,
                });
            }
        }
    }
    pub(super) fn due(&self, now: Instant) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.due <= now)
            .min_by_key(|(_, entry)| entry.due)
            .map(|(index, _)| index)
    }
    pub(super) fn settle(&mut self, ticket: &Ticket, now: Instant, success: bool) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.ticket.same_record(ticket))
        {
            entry.failures = if success {
                0
            } else {
                entry.failures.saturating_add(1)
            };
            let millis = if success {
                750
            } else {
                (500u64 << entry.failures.min(6)).min(30_000)
            };
            entry.due = now + Duration::from_millis(millis);
        }
    }
}
