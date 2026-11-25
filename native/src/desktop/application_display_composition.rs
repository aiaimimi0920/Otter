//! 单应用单 worker 的显示合成缓存；无任务队列，完成通知唤醒，过期结果不发布。
use crate::{
    desktop::{
        display_composition,
        image::Image,
        loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
    },
    editor::{DocumentStamp, Editor},
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
#[path = "application_display_live_cache.rs"]
mod live;

struct Key {
    source: UnitId,
    origin: Origin,
}
enum Origin {
    Document { image: Image, stamp: DocumentStamp },
    Live(Arc<()>),
}
impl Key {
    fn matches(&self, source: &UnitId, image: &Image, editor: &Editor) -> bool {
        self.source == *source
            && matches!(&self.origin, Origin::Document { image: old, stamp }
            if stamp.matches(editor) && old.width == image.width && old.height == image.height
                && old.pixels.same_storage(&image.pixels))
    }
}
struct Entry {
    key: Key,
    result: Result<Image, ()>,
}
struct Running {
    key: Key,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: JoinHandle<Result<Image, ()>>,
}
#[derive(Default)]
pub(super) struct Cache {
    entries: Vec<Entry>,
    running: Option<Running>,
    live_refreshes: Vec<live::Refresh>,
    live_updates: Vec<(UnitId, Arc<()>)>,
}
impl Cache {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        reap_deadline(&self.running.as_ref()?.ready)
    }
    /// 替换工作区或发布新来源时撤销旧结果；仍占用原单槽，直到线程退出。
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.live_refreshes.clear();
        self.live_updates.clear();
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    pub fn poll(&mut self) -> bool {
        if !self
            .running
            .as_ref()
            .is_some_and(|r| r.worker.is_finished())
        {
            return false;
        }
        let running = self.running.take().unwrap();
        let result = running.worker.join().unwrap_or(Err(()));
        if !running.cancel.load(Ordering::Acquire)
            && self.accept_live_result(&running.key, result.is_ok())
        {
            self.entries.push(Entry {
                key: running.key,
                result,
            });
        }
        true
    }
    /// 清除关闭或改变的来源；只取消，不在 UI 更新路径等待正在运行的原生调用。
    pub fn retain(&mut self, mut current: impl FnMut(&UnitId, &Image, &DocumentStamp) -> bool) {
        let mut keep = |key: &Key| match &key.origin {
            Origin::Document { image, stamp } => current(&key.source, image, stamp),
            Origin::Live(_) => true,
        };
        self.entries.retain(|entry| keep(&entry.key));
        if let Some(running) = &self.running {
            if !keep(&running.key) {
                running.cancel.store(true, Ordering::Release);
            }
        }
    }
    /// None 表示等待现有任务或新任务；Err 在相同来源版本下缓存，避免失败循环。
    pub fn request(
        &mut self,
        source: &UnitId,
        image: &Image,
        editor: &Editor,
        notify: Notify,
    ) -> Result<Option<Image>, ()> {
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| e.key.matches(source, image, editor))
        {
            return entry.result.clone().map(Some);
        }
        if self.running.is_some() {
            return Ok(None);
        }
        let key = Key {
            source: source.clone(),
            origin: Origin::Document {
                image: image.clone(),
                stamp: editor.document_stamp(),
            },
        };
        let snapshot = editor.export_snapshot();
        let input = image.clone();
        self.start(key, notify, move |signal| {
            display_composition::compose(&input, &snapshot, signal).map_err(|error| {
                eprintln!("Display composition failed: {error}");
            })
        })
    }
    fn start(
        &mut self,
        key: Key,
        notify: Notify,
        prepare: impl FnOnce(&AtomicBool) -> Result<Image, ()> + Send + 'static,
    ) -> Result<Option<Image>, ()> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        match std::thread::Builder::new()
            .name("otter-display-compose".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                prepare(&signal)
            }) {
            Ok(worker) => {
                self.running = Some(Running {
                    key,
                    cancel,
                    ready,
                    worker,
                });
                Ok(None)
            }
            Err(error) => {
                eprintln!("Display composition worker failed: {error}");
                if self.accept_live_result(&key, false) {
                    self.entries.push(Entry {
                        key,
                        result: Err(()),
                    });
                }
                Err(())
            }
        }
    }
}
impl Drop for Cache {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            running.cancel.store(true, Ordering::Release);
            let _ = running.worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::loom_execution_tasks::reap_fixture::{wait_finished, Gate, WAIT};
    #[test]
    fn completed_source_reuses_pixels_and_retirement_discards_cancelled_result() {
        let source = UnitId::parse("source").unwrap();
        let image = Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        };
        let editor = Editor::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let notify: Notify = Arc::new(move || {
            tx.send(()).unwrap();
        });
        let mut cache = Cache::default();
        assert!(cache
            .request(&source, &image, &editor, notify.clone())
            .unwrap()
            .is_none());
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        // 已完成但尚未交付的结果也必须服从断连/关闭取消。
        cache.retain(|_, _, _| false);
        wait_finished(&cache.running.as_ref().unwrap().worker);
        assert!(cache.poll());
        assert!(cache.entries.is_empty());
        assert!(cache
            .request(&source, &image, &editor, notify.clone())
            .unwrap()
            .is_none());
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        wait_finished(&cache.running.as_ref().unwrap().worker);
        assert!(cache.poll());
        let output = cache
            .request(&source, &image, &editor, notify.clone())
            .unwrap()
            .unwrap();
        assert!(output.pixels.same_storage(&image.pixels));
        assert!(cache.running.is_none());
        assert_eq!(cache.entries.len(), 1);
        assert!(rx.try_recv().is_err());
        let replacement = Editor::default();
        assert!(cache
            .request(&source, &image, &replacement, notify)
            .unwrap()
            .is_none());
        cache.retain(|_, _, stamp| stamp.matches(&replacement));
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn notification_tail_keeps_the_slot_until_exit_and_invalidation_discards_result() {
        let source = UnitId::parse("source").unwrap();
        let replacement = UnitId::parse("replacement").unwrap();
        let image = Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        };
        let editor = Editor::default();
        for invalidate in [false, true] {
            let mut cache = Cache::default();
            let gate = Gate::new(|| {});
            assert!(cache.deadline().is_none());
            assert!(cache
                .request(&source, &image, &editor, gate.notify.clone())
                .unwrap()
                .is_none());
            gate.noticed.recv_timeout(WAIT).unwrap();
            assert!(!cache.poll());
            assert!(cache.deadline().is_some());
            if invalidate {
                cache.invalidate();
                assert!(cache
                    .request(&replacement, &image, &editor, gate.notify.clone())
                    .unwrap()
                    .is_none());
                assert_eq!(cache.running.as_ref().unwrap().key.source, source);
            }
            assert!(cache.is_busy());
            assert!(cache.entries.is_empty());
            gate.release.send(()).unwrap();
            wait_finished(&cache.running.as_ref().unwrap().worker);
            assert!(cache.poll());
            assert!(!cache.poll());
            assert!(!cache.is_busy());
            assert!(cache.deadline().is_none());
            assert_eq!(cache.entries.len(), usize::from(!invalidate));
            assert!(gate.noticed.try_recv().is_err());
        }
    }
}
