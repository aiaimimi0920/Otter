//! 冻结库菜单索引 worker：只读不可变原文，输出有界文本，不解码图片或授权文件。
use crate::{
    session_library::{self, Kind},
    session_source::Source,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
pub(super) struct Entry {
    pub id: String,
    pub label: String,
}
pub(super) type Index = [Result<Vec<Entry>, String>; 2];
pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Index>>,
}
impl Job {
    pub fn start(source: Source) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("otter-frozen-menu".into())
            .spawn(move || {
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
            })?;
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }
    pub fn poll(&mut self) -> Option<Index> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .unwrap_or_else(|_| [Err("Worker failed".into()), Err("Worker failed".into())]),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // 只有有界 JSON 解析，没有 WIC/文件/网络调用；取消后回收本次唯一 worker。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
