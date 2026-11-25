//! 冻结库菜单索引 worker：只读不可变原文，输出有界文本，不解码图片或授权文件。
use super::worker::{self, Slot};
use crate::{
    desktop::loom_execution_signal::Notify,
    session_library::{self, Kind},
    session_source::Source,
};
use std::sync::atomic::Ordering;
pub(super) struct Entry {
    pub id: String,
    pub label: String,
    pub source_unit: String,
}
pub(super) type Index = [Result<Vec<Entry>, String>; 2];
pub(super) struct Job(worker::Job<Index>);
impl Job {
    pub fn start(
        slot: &Slot<Index>,
        source: Source,
        notify: Option<Notify>,
    ) -> std::io::Result<Option<Self>> {
        slot.start("otter-frozen-menu", notify, move |signal| {
            [Kind::RecycleBin, Kind::ReferenceLibrary].map(|kind| {
                if signal.load(Ordering::Acquire) {
                    return Err("Cancelled".into());
                }
                let library = session_library::decode(&source, kind)?;
                let mut entries = Vec::with_capacity(library.entries().len());
                for entry in library.entries() {
                    if signal.load(Ordering::Acquire) {
                        return Err("Cancelled".into());
                    }
                    let date: String = entry
                        .created_at
                        .chars()
                        .take(48)
                        .map(|ch| if ch.is_control() { ' ' } else { ch })
                        .collect();
                    entries.push(Entry {
                        id: entry.entry_id.to_string(),
                        source_unit: entry.source_sticker_id.to_string(),
                        label: format!(
                            "{} × {} · {}",
                            entry.window.w,
                            entry.window.h,
                            date.replace('&', "&&")
                        ),
                    });
                }
                Ok(entries)
            })
        })
        .map(|job| job.map(Self))
    }
    pub fn poll(&mut self) -> Option<Index> {
        Some(
            self.0
                .poll()?
                .unwrap_or_else(|_| [Err("Worker failed".into()), Err("Worker failed".into())]),
        )
    }
}
