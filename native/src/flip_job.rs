//! 翻转的单任务后台生命周期；取消、失败或销毁不交付部分栅格和几何。
use crate::editor::{FlipError, FlipPlan, RasterRefreshPlan};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum Error {
    Busy,
    Cancelled,
    Compute(FlipError),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<Result<RasterRefreshPlan, Error>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}
impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn start(&mut self, plan: FlipPlan) -> Result<(), Error> {
        if self.is_busy() {
            return Err(Error::Busy);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = thread::Builder::new()
            .name("otter-flip".into())
            .spawn(move || {
                plan.compute(|| signal.load(Ordering::Acquire))
                    .map_err(Error::Compute)
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running { cancel, worker });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    /// 不阻塞消息线程；即使计算先完成，已取消的候选也不能提交。
    /// 文档身份、版本与源图校验由 Editor 的原子提交入口负责。
    pub fn poll(&mut self) -> Option<Result<RasterRefreshPlan, Error>> {
        if !self
            .running
            .as_ref()
            .is_some_and(|r| r.worker.is_finished())
        {
            return None;
        }
        let running = self.running.take().unwrap();
        let result = running.worker.join().unwrap_or(Err(Error::WorkerFailed));
        Some(if running.cancel.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(running) = self.running.take() {
            let _ = running.worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{Editor, Tool},
        erase_layers::Layers,
        raster_flip::Axis,
        viewport::Point,
    };
    fn document() -> Editor {
        Editor::with_layers(Layers {
            annotation_size: None,
            size: (2, 1),
            base: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
            annotations: None,
        })
        .unwrap()
    }
    fn plan(editor: &Editor) -> FlipPlan {
        editor
            .prepare_flip(Axis::Horizontal, |_| panic!("no text"))
            .unwrap()
            .unwrap()
    }
    fn finish(job: &mut Job) -> Result<RasterRefreshPlan, Error> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(result) = job.poll() {
                return result;
            }
            assert!(std::time::Instant::now() < deadline, "flip worker timeout");
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn single_flight_is_private_until_atomic_commit_and_reusable() {
        let mut editor = document();
        let version = editor.revision();
        let mut job = Job::default();
        assert!(job.poll().is_none());
        job.start(plan(&editor)).unwrap();
        assert!(matches!(job.start(plan(&editor)), Err(Error::Busy)));
        let ready = finish(&mut job).unwrap();
        assert_eq!(editor.revision(), version);
        assert!(!job.is_busy());
        assert!(editor.commit_raster_refresh(ready).unwrap());
        assert_eq!(editor.revision(), version + 1);
        assert!(editor.undo());
        assert!(editor.redo());
        job.start(plan(&editor)).unwrap();
        assert!(editor
            .commit_raster_refresh(finish(&mut job).unwrap())
            .unwrap());
    }
    #[test]
    fn cancellation_suppresses_completed_result_and_allows_restart() {
        let editor = document();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !job.running.as_ref().unwrap().worker.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(1));
        }
        job.cancel();
        assert!(matches!(finish(&mut job), Err(Error::Cancelled)));
        assert!(!job.is_busy());
        job.start(plan(&editor)).unwrap();
        assert!(finish(&mut job).is_ok());
    }
    #[test]
    fn stale_and_foreign_document_results_cannot_commit() {
        let mut editor = document();
        let mut other = document();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        assert!(!other
            .commit_raster_refresh(finish(&mut job).unwrap())
            .unwrap());
        job.start(plan(&editor)).unwrap();
        editor.begin(Tool::Line, Point { x: 0.0, y: 0.0 }).unwrap();
        editor.extend(Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(editor.commit());
        assert!(!editor
            .commit_raster_refresh(finish(&mut job).unwrap())
            .unwrap());
        assert!(editor.undo());
        assert!(editor.redo());
    }
    #[test]
    fn drop_joins_without_publishing() {
        let editor = document();
        let version = editor.revision();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        drop(job);
        assert_eq!(editor.revision(), version);
    }
}
