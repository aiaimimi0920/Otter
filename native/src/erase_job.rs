//! 每个文档只允许一个擦除提交任务；不排队也不覆盖手势，关闭时取消并回收线程。
use crate::{
    erase::EraseError,
    erase_layers::{Layers, Mode},
    viewport::Point,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
};

#[derive(Clone)]
pub struct Request {
    /// 一次手势的进程内身份；宿主路径只追加，克隆提交请求保留同一身份。
    pub preview_identity: Arc<()>,
    /// 手势开始时的标注层目标框；路径仍为文档源坐标。
    pub annotation_frame: Option<crate::crop_geometry::Rect>,
    pub base_projection: Option<crate::image_projection::Projection>,
    pub source: Layers,
    pub revision: u64,
    pub mode: Mode,
    pub points: Vec<Point>,
    pub width: f32,
}
pub struct Completed {
    pub source: Layers,
    pub revision: u64,
    pub result: Result<Layers, JobError>,
    pub refresh: Option<crate::editor::RasterRefreshPlan>,
}
#[derive(Debug)]
pub enum JobError {
    Busy,
    InvalidInput,
    Compute(EraseError),
    Refresh(crate::blur::BlurError),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    source: Layers,
    revision: u64,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
    receiver: Receiver<WorkerResult>,
}
type WorkerResult = Result<(Layers, Option<crate::editor::RasterRefreshPlan>), JobError>;
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}

impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn is_current(&self, source: &Layers, revision: u64) -> bool {
        self.running
            .as_ref()
            .is_some_and(|r| r.revision == revision && r.source.same_storage(source))
    }
    pub fn start(&mut self, request: Request) -> Result<(), JobError> {
        self.start_inner(request, None)
    }
    pub fn start_edit(
        &mut self,
        request: Request,
        plan: crate::editor::RasterRefreshPlan,
    ) -> Result<(), JobError> {
        if !plan.matches_source(&request.source, request.revision) {
            return Err(JobError::InvalidInput);
        }
        self.start_inner(request, Some(plan))
    }
    fn start_inner(
        &mut self,
        request: Request,
        plan: Option<crate::editor::RasterRefreshPlan>,
    ) -> Result<(), JobError> {
        if self.is_busy() {
            return Err(JobError::Busy);
        }
        if request.points.is_empty()
            || request.points.len() > 65_536
            || !request.width.is_finite()
            || !(4.0..=96.0).contains(&request.width)
            || request.points.iter().any(|p| {
                !p.x.is_finite()
                    || !p.y.is_finite()
                    || p.x.abs() > 1_000_000.0
                    || p.y.abs() > 1_000_000.0
            })
        {
            return Err(JobError::InvalidInput);
        }
        let source = request.source.clone();
        let revision = request.revision;
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("otter-erase".into())
            .spawn(move || {
                let result = request
                    .source
                    .erase_in_frames(
                        request.mode,
                        &request.points,
                        request.width,
                        request.base_projection,
                        request.annotation_frame,
                        || signal.load(Ordering::Acquire),
                    )
                    .map_err(JobError::Compute)
                    .and_then(|layers| {
                        let refresh = plan
                            .map(|plan| {
                                plan.with_candidate(layers.clone())?
                                    .compute(|| signal.load(Ordering::Acquire))
                            })
                            .transpose()
                            .map_err(JobError::Refresh)?;
                        Ok((layers, refresh))
                    });
                let _ = sender.send(result);
            })
            .map_err(JobError::Spawn)?;
        self.running = Some(Running {
            source,
            revision,
            cancel,
            handle,
            receiver,
        });
        Ok(())
    }
    pub fn cancel(&mut self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    /// 不阻塞消息线程；只有线程完成后才 join，取消的结果不提交。
    pub fn poll(&mut self) -> Option<Completed> {
        if !self
            .running
            .as_ref()
            .is_some_and(|r| r.handle.is_finished())
        {
            return None;
        }
        let running = self.running.take().unwrap();
        let joined = running.handle.join();
        let result = match (joined, running.receiver.try_recv()) {
            (Ok(()), Ok(result)) => result,
            _ => Err(JobError::WorkerFailed),
        };
        if running.cancel.load(Ordering::Acquire) {
            return None;
        }
        let (result, refresh) = match result {
            Ok((layers, refresh)) => (Ok(layers), refresh),
            Err(error) => (Err(error), None),
        };
        Some(Completed {
            source: running.source,
            revision: running.revision,
            result,
            refresh,
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(running) = self.running.take() {
            let _ = running.handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request {
        Request {
            preview_identity: Arc::new(()),
            annotation_frame: None,
            base_projection: None,
            source: Layers {
                annotation_size: None,
                size: (32, 32),
                base: vec![255; 32 * 32 * 4].into(),
                annotations: None,
            },
            revision: 12,
            mode: Mode::Content,
            points: vec![Point { x: 16.0, y: 16.0 }],
            width: 8.0,
        }
    }
    fn finish(job: &mut Job) -> Option<Completed> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while job.is_busy() {
            if let Some(result) = job.poll() {
                return Some(result);
            }
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(1));
        }
        None
    }
    #[test]
    fn cropped_worker_erases_base_and_annotation_at_distinct_coordinates() {
        for mode in [Mode::Content, Mode::RasterAnnotations] {
            let mut work = request();
            work.mode = mode;
            work.source.annotations = Some([64, 64, 64, 128].repeat(32 * 32).into());
            work.annotation_frame = Some(crate::crop_geometry::Rect {
                x: 16.0,
                y: 4.0,
                width: 8.0,
                height: 16.0,
            });
            work.points = vec![Point { x: 20.0, y: 12.0 }];
            work.width = 4.0;
            let source = work.source.clone();
            let mut job = Job::default();
            job.start(work).unwrap();
            let layers = finish(&mut job).unwrap().result.unwrap();
            let overlay = layers.annotations.as_ref().unwrap();
            // 源坐标 (20,12) 对应标注纹理中心 (16,16)，而非底图的 (20,12)。
            assert_eq!(overlay[(16 * 32 + 16) * 4 + 3], 0);
            assert_eq!(overlay[(8 * 32 + 16) * 4 + 3], 128);
            assert_eq!(
                layers.base[(12 * 32 + 20) * 4 + 3],
                if mode == Mode::Content { 0 } else { 255 }
            );
            assert_eq!(layers.base[(16 * 32 + 16) * 4 + 3], 255);
            if mode == Mode::RasterAnnotations {
                assert!(layers.base.same_storage(&source.base));
            }
            assert_eq!(
                source.annotations.as_ref().unwrap()[(16 * 32 + 16) * 4 + 3],
                128
            );
        }
    }

    #[test]
    fn worker_completes_and_busy_request_cannot_overwrite_active_stroke() {
        let mut job = Job::default();
        let work = request();
        let source = work.source.clone();
        job.start(work).unwrap();
        assert!(job.is_current(&source, 12));
        assert!(!job.is_current(&source, 13));
        assert!(matches!(job.start(request()), Err(JobError::Busy)));
        let completed = finish(&mut job).unwrap();
        assert_eq!(completed.revision, 12);
        assert!(completed.source.same_storage(&source));
        assert_eq!(completed.result.unwrap().base[(16 * 32 + 16) * 4 + 3], 0);
    }
    #[test]
    fn cancel_drop_and_invalid_request_release_sources_and_allow_reuse() {
        let mut job = Job::default();
        let mut work = request();
        work.points.clear();
        assert!(matches!(job.start(work), Err(JobError::InvalidInput)));
        assert!(!job.is_busy());
        let work = request();
        let weak = work.source.base.downgrade();
        job.start(work).unwrap();
        job.cancel();
        assert!(finish(&mut job).is_none());
        assert!(weak.upgrade().is_none());
        let work = request();
        let weak = work.source.base.downgrade();
        job.start(work).unwrap();
        drop(job);
        assert!(weak.upgrade().is_none());
    }
}
