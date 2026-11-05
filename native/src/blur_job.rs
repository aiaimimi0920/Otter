//! 单工作线程、单待处理请求；新请求取消旧计算，过期结果不交付。
//! 宿主轮询仅检查完成状态，Drop 先取消再 join，绝不遗留计算线程。
use crate::{
    blur::{self, BlurError, Region},
    erase_layers::Layers,
    pixels::Pixels,
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
    /// 世代凭证原样往返工作线程；宿主提交前必须验证。
    pub(crate) stamp: crate::editor::DocumentStamp,
    /// None 准备后续笔迹；Some 更新指定索引，宿主仍须核验版本与选择。
    pub target: Option<usize>,
    pub annotation_frame: Option<crate::crop_geometry::Rect>,
    pub base_projection: Option<crate::image_projection::Projection>,
    pub source: Layers,
    pub size: (u32, u32),
    pub area: Region,
    pub sigma: f32,
    /// 宿主文档版本；提交结果时还须核验 source.same_storage。
    pub revision: u64,
}
pub struct Completed {
    pub request: Request,
    pub result: Result<Pixels, JobError>,
}
#[derive(Debug)]
pub enum JobError {
    Compute(BlurError),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    request: Request,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
    receiver: Receiver<Result<Pixels, BlurError>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
    pending: Option<Request>,
}

impl Job {
    pub fn current_request(&self) -> Option<&Request> {
        self.pending
            .as_ref()
            .or_else(|| self.running.as_ref().map(|r| &r.request))
    }
    pub fn is_current(&self, source: &Layers, revision: u64) -> bool {
        self.pending
            .as_ref()
            .or_else(|| self.running.as_ref().map(|running| &running.request))
            .is_some_and(|request| {
                request.revision == revision && request.source.same_storage(source)
            })
    }
    pub fn is_busy(&self) -> bool {
        self.running.is_some() || self.pending.is_some()
    }
    /// 请求不复制源像素；替换 pending 会立即释放其快照引用。
    pub fn request(&mut self, request: Request) -> Result<(), JobError> {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
            self.pending = Some(request);
            Ok(())
        } else {
            self.start(request)
        }
    }
    fn start(&mut self, request: Request) -> Result<(), JobError> {
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let work = request.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("otter-blur".into())
            .spawn(move || {
                // 合成与卷积都在同一个可取消工作线程中，UI 只持有共享快照。
                let cancelled = || worker_cancel.load(Ordering::Acquire);
                let result = if work.size != work.source.size {
                    Err(BlurError::InvalidInput)
                } else {
                    crate::raster_source::compose_in_frames(
                        &work.source,
                        work.base_projection,
                        work.annotation_frame,
                        cancelled,
                    )
                    .and_then(|source| {
                        blur::region_precise(&source, work.size, work.area, work.sigma, cancelled)
                    })
                };
                let _ = sender.send(result);
            })
            .map_err(JobError::Spawn)?;
        self.running = Some(Running {
            request,
            cancel,
            handle,
            receiver,
        });
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.pending = None;
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    /// 不等待未完成线程。新请求的计算只在旧线程真正结束后开始。
    pub fn poll(&mut self) -> Option<Completed> {
        if !self
            .running
            .as_ref()
            .is_some_and(|running| running.handle.is_finished())
        {
            return None;
        }
        let running = self.running.take().unwrap();
        let obsolete = running.cancel.load(Ordering::Acquire);
        let joined = running.handle.join();
        let result = match (joined, running.receiver.try_recv()) {
            (Ok(()), Ok(result)) => result.map_err(JobError::Compute),
            _ => Err(JobError::WorkerFailed),
        };
        if let Some(next) = self.pending.take() {
            // 释放旧结果/源图后再启动，避免无用快照挤占新请求的像素预算。
            drop(result);
            drop(running.request);
            let request = next.clone();
            if let Err(error) = self.start(next) {
                return Some(Completed {
                    request,
                    result: Err(error),
                });
            }
            return None;
        }
        if obsolete {
            None
        } else {
            Some(Completed {
                request: running.request,
                result,
            })
        }
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
    fn request(revision: u64) -> Request {
        Request {
            stamp: crate::editor::Editor::default().document_stamp(),
            target: None,
            annotation_frame: None,
            base_projection: None,
            source: Layers {
                annotation_size: None,
                size: (32, 32),
                base: vec![255; 32 * 32 * 4].into(),
                annotations: None,
            },
            size: (32, 32),
            area: Region {
                x: 0,
                y: 0,
                width: 32,
                height: 32,
            },
            sigma: 2.0,
            revision,
        }
    }
    fn drain(job: &mut Job) -> Vec<Completed> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut results = Vec::new();
        while job.is_busy() {
            if let Some(result) = job.poll() {
                results.push(result);
            }
            assert!(
                std::time::Instant::now() < deadline,
                "blur worker did not finish"
            );
            thread::sleep(std::time::Duration::from_millis(1));
        }
        results
    }
    #[test]
    fn latest_request_wins_and_obsolete_source_is_released() {
        let mut job = Job::default();
        let first = request(1);
        let old_source = first.source.base.downgrade();
        job.request(first).unwrap();
        let replaced = request(2);
        let replaced_source = replaced.source.base.downgrade();
        job.request(replaced).unwrap();
        let latest = request(3);
        let expected = latest.source.clone();
        job.request(latest).unwrap();
        assert!(replaced_source.upgrade().is_none());
        let mut results = drain(&mut job);
        assert_eq!(results.len(), 1);
        let result = results.pop().unwrap();
        assert_eq!(result.request.revision, 3);
        assert!(result.request.source.same_storage(&expected));
        assert!(result.result.is_ok());
        assert!(old_source.upgrade().is_none());
    }
    #[test]
    fn cancellation_drops_pending_and_does_not_publish_completed_old_work() {
        let mut job = Job::default();
        job.request(request(1)).unwrap();
        job.request(request(2)).unwrap();
        job.cancel();
        assert!(drain(&mut job).is_empty());
        job.request(request(3)).unwrap();
        assert_eq!(drain(&mut job).len(), 1);
    }
    #[test]
    fn drop_joins_worker_and_releases_all_source_references() {
        let mut job = Job::default();
        let first = request(1);
        let weak = first.source.base.downgrade();
        job.request(first).unwrap();
        drop(job);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn worker_composes_raster_annotations_and_checks_both_source_identities() {
        let mut work = request(7);
        work.source.base = [0, 0, 255, 255].repeat(32 * 32).into();
        work.source.annotations = Some([255, 0, 0, 255].repeat(32 * 32).into());
        let source = work.source.clone();
        let expected = blur::region_precise(
            source.annotations.as_ref().unwrap(),
            work.size,
            work.area,
            work.sigma,
            || false,
        )
        .unwrap();
        let overlay = source.annotations.as_ref().unwrap().downgrade();
        let mut job = Job::default();
        job.request(work).unwrap();
        assert!(job.is_current(&source, 7));
        let mut changed = source.clone();
        changed.annotations = Some([255, 0, 0, 255].repeat(32 * 32).into());
        assert!(!job.is_current(&changed, 7));
        assert!(!job.is_current(&source, 8));
        let results = drain(&mut job);
        let result = results[0].result.as_ref().unwrap();
        assert_eq!(&result[..], &expected[..]);
        assert_eq!(
            &result[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4],
            &[255, 0, 0, 255]
        );
        assert_eq!(&source.base[..4], &[0, 0, 255, 255]);
        drop(results);
        drop(source);
        assert!(overlay.upgrade().is_none());
    }

    #[test]
    fn invalid_request_is_reported_and_next_request_still_runs() {
        let mut job = Job::default();
        let mut invalid = request(1);
        invalid.sigma = 0.0;
        job.request(invalid).unwrap();
        let results = drain(&mut job);
        assert!(matches!(
            results[0].result,
            Err(JobError::Compute(BlurError::InvalidInput))
        ));
        job.request(request(2)).unwrap();
        assert!(drain(&mut job)[0].result.is_ok());
    }
}
