//! 组缩放效果的单任务后台计算；无队列，同强度共享结果，取消/失败不交付部分候选。
use crate::{blur, blur_image::BlurImage, editor::ResizePlan};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum Error {
    InvalidInput,
    Busy,
    Cancelled,
    Compute(blur::BlurError),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<Result<ResizePlan, Error>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}

impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn start(&mut self, plan: ResizePlan) -> Result<(), Error> {
        if self.is_busy() {
            return Err(Error::Busy);
        }
        let targets = plan.blur_targets().map_err(|_| Error::InvalidInput)?;
        if !targets.is_empty() && plan.source_layers().is_none() {
            return Err(Error::InvalidInput);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = thread::Builder::new()
            .name("otter-resize".into())
            .spawn(move || compute(plan, targets, || signal.load(Ordering::Acquire)))
            .map_err(Error::Spawn)?;
        self.running = Some(Running { cancel, worker });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    /// 只回收已结束线程；取消后的结果即使计算成功也不交付。
    pub fn poll(&mut self) -> Option<Result<ResizePlan, Error>> {
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

fn compute(
    mut plan: ResizePlan,
    targets: Vec<(usize, f32)>,
    cancelled: impl Fn() -> bool,
) -> Result<ResizePlan, Error> {
    let source = if targets.is_empty() {
        None
    } else {
        Some(
            crate::raster_source::compose_in_frames(
                plan.source_layers().ok_or(Error::InvalidInput)?,
                plan.base_projection(),
                plan.annotation_frame(),
                &cancelled,
            )
            .map_err(Error::Compute)?,
        )
    };
    let mut images = BTreeMap::new();
    for (index, sigma) in targets {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        let image = if let Some(image) = images.get(&sigma.to_bits()) {
            BlurImage::clone(image)
        } else {
            let layers = plan.source_layers().ok_or(Error::InvalidInput)?;
            let (width, height) = layers.size;
            let pixels = blur::region_precise(
                source.as_ref().ok_or(Error::InvalidInput)?,
                layers.size,
                blur::Region {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                sigma,
                &cancelled,
            )
            .map_err(Error::Compute)?;
            let image = BlurImage::with_strength(width, height, pixels, sigma)
                .ok_or(Error::InvalidInput)?;
            images.insert(sigma.to_bits(), image.clone());
            image
        };
        plan.replace_blur(index, image)
            .map_err(|_| Error::InvalidInput)?;
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Editor, erase_layers::Layers, viewport::Point};
    fn editor() -> Editor {
        let pixels: crate::pixels::Pixels = vec![255; 16 * 16 * 4].into();
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (16, 16),
            base: pixels.clone(),
            annotations: None,
        })
        .unwrap();
        for x in [4.0, 10.0] {
            editor
                .begin_blur(
                    Point { x, y: 8.0 },
                    BlurImage::new(16, 16, pixels.clone(), 4).unwrap(),
                )
                .unwrap();
            assert!(editor.commit());
        }
        editor
    }
    fn plan(editor: &Editor) -> ResizePlan {
        editor
            .prepare_resize_many(
                &[0, 1],
                Point { x: 8.0, y: 8.0 },
                Point { x: 2.0, y: 2.0 },
                |_| unreachable!(),
            )
            .unwrap()
            .unwrap()
    }
    fn finish(job: &mut Job) -> Result<ResizePlan, Error> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = job.poll() {
                return result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn cropped_resize_samples_visible_overlay_and_restores_effects_with_geometry() {
        let mut editor = editor();
        let old_blur = editor.elements()[0].blur.as_ref().unwrap().clone();
        let source = editor.layers().unwrap().clone();
        let mut layers = source.clone();
        layers.annotations = Some(
            (0..256)
                .flat_map(|i| {
                    if i % 16 < 8 {
                        [0, 0, 255, 255]
                    } else {
                        [0, 255, 0, 255]
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        );
        assert!(editor
            .replace_layers(editor.revision(), &source, layers)
            .unwrap());
        let crop = crate::crop_geometry::Rect {
            x: 8.0,
            y: 2.0,
            width: 8.0,
            height: 6.0,
        };
        assert!(editor.crop_to(crop).unwrap());
        let before_crop = editor.crop_view();
        let before_geometry: Vec<_> = editor.elements().iter().map(|s| s.points.clone()).collect();
        let prepared = plan(&editor);
        assert_eq!(prepared.annotation_frame(), Some(crop));
        let composed =
            crate::raster_source::compose_in_frame(editor.layers().unwrap(), Some(crop), || false)
                .unwrap();
        let expected = blur::region_precise(
            &composed,
            (16, 16),
            blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            8.0,
            || false,
        )
        .unwrap();
        let mut job = Job::default();
        let revision = editor.revision();
        job.start(prepared).unwrap();
        let ready = finish(&mut job).unwrap();
        assert_eq!(editor.revision(), revision);
        assert!(editor.commit_resize(ready).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.crop_view(), before_crop);
        for stroke in editor.elements() {
            assert_eq!(&stroke.blur.as_ref().unwrap().pixels()[..], &expected[..]);
        }
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(editor.elements()[1].blur.as_ref().unwrap().pixels()));
        assert!(editor.undo());
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old_blur.pixels()));
        for (stroke, points) in editor.elements().iter().zip(&before_geometry) {
            assert_eq!(&stroke.points, points);
        }
        assert!(editor.redo());
        assert_eq!(
            &editor.elements()[0].blur.as_ref().unwrap().pixels()[..],
            &expected[..]
        );
    }
    #[test]
    fn crop_reset_during_resize_rejects_finished_plan_and_preserves_history() {
        let mut editor = editor();
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 2.0,
                y: 2.0,
                width: 12.0,
                height: 12.0,
            })
            .unwrap();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        assert!(editor.reset_crop());
        assert!(!editor.commit_resize(finish(&mut job).unwrap()).unwrap());
        assert!(editor.undo());
        assert!(editor.crop_view().is_some());
        assert!(editor.redo());
        assert!(editor.crop_view().is_none());
        assert_eq!(editor.elements()[0].blur.as_ref().unwrap().strength(), 4.0);
    }

    #[test]
    fn worker_deduplicates_strength_and_commits_geometry_and_effects_once() {
        let mut editor = editor();
        let revision = editor.revision();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        assert!(matches!(job.start(plan(&editor)), Err(Error::Busy)));
        let result = finish(&mut job).unwrap();
        assert_eq!(editor.revision(), revision);
        assert!(editor.commit_resize(result).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        let a = editor.elements()[0].blur.as_ref().unwrap();
        let b = editor.elements()[1].blur.as_ref().unwrap();
        assert_eq!(a.sigma(), 8);
        assert!(a.pixels().same_storage(b.pixels()));
        let source = editor.layers().unwrap();
        let expected = blur::region(
            &source.base,
            source.size,
            blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            8,
            || false,
        )
        .unwrap();
        assert_eq!(&a.pixels()[..], &expected[..]);
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].blur.as_ref().unwrap().sigma(), 4);
        assert_eq!(editor.elements()[0].points[0].x, 4.0);
    }
    #[test]
    fn cancellation_and_stale_completion_never_change_document() {
        let mut editor = editor();
        let revision = editor.revision();
        let mut job = Job::default();
        job.start(plan(&editor)).unwrap();
        job.cancel();
        assert!(matches!(finish(&mut job), Err(Error::Cancelled)));
        assert_eq!(editor.revision(), revision);
        job.start(plan(&editor)).unwrap();
        assert!(editor.undo());
        assert!(!editor.commit_resize(finish(&mut job).unwrap()).unwrap());
        assert_eq!(editor.elements().len(), 1);
    }
    #[test]
    fn unsupported_strength_rejected_before_worker_and_job_remains_reusable() {
        let editor = editor();
        let invalid = editor
            .prepare_resize_many(
                &[0, 1],
                Point::default(),
                Point { x: 20.0, y: 20.0 },
                |_| unreachable!(),
            )
            .unwrap()
            .unwrap();
        let mut job = Job::default();
        assert!(matches!(job.start(invalid), Err(Error::InvalidInput)));
        assert!(!job.is_busy());
        job.start(plan(&editor)).unwrap();
        assert!(finish(&mut job).is_ok());
    }
    #[test]
    fn repeated_small_resizes_preserve_fractional_strength_and_history() {
        let mut editor = editor();
        let mut job = Job::default();
        for expected in [4.4, 4.84] {
            let plan = editor
                .prepare_resize_many(
                    &[0, 1],
                    crate::viewport::Point::default(),
                    crate::viewport::Point { x: 1.1, y: 1.1 },
                    |_| unreachable!(),
                )
                .unwrap()
                .unwrap();
            job.start(plan).unwrap();
            assert!(editor.commit_resize(finish(&mut job).unwrap()).unwrap());
            let a = editor.elements()[0].blur.as_ref().unwrap();
            assert!((a.strength() - expected).abs() < 0.00001);
            assert!(a
                .pixels()
                .same_storage(editor.elements()[1].blur.as_ref().unwrap().pixels()));
        }
        assert!(editor.undo());
        assert!((editor.elements()[0].blur.as_ref().unwrap().strength() - 4.4).abs() < 0.00001);
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].blur.as_ref().unwrap().strength(), 4.0);
    }
    #[test]
    fn resized_blur_samples_raster_annotation_overlay_not_only_base() {
        let base: crate::pixels::Pixels = [0, 0, 255, 255].repeat(16 * 16).into();
        let overlay: crate::pixels::Pixels = [255, 0, 0, 255].repeat(16 * 16).into();
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (16, 16),
            base: base.clone(),
            annotations: Some(overlay.clone()),
        })
        .unwrap();
        editor
            .begin_blur(
                Point { x: 8.0, y: 8.0 },
                BlurImage::new(16, 16, base.clone(), 4).unwrap(),
            )
            .unwrap();
        editor.commit();
        let plan = editor
            .prepare_resize_many(
                &[0],
                Point { x: 8.0, y: 8.0 },
                Point { x: 1.25, y: 1.25 },
                |_| unreachable!(),
            )
            .unwrap()
            .unwrap();
        let mut job = Job::default();
        job.start(plan).unwrap();
        assert!(editor.commit_resize(finish(&mut job).unwrap()).unwrap());
        let image = editor.elements()[0].blur.as_ref().unwrap();
        let expected = blur::region(
            &overlay,
            (16, 16),
            blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            5,
            || false,
        )
        .unwrap();
        assert_eq!(&image.pixels()[..], &expected[..]);
        assert!(editor.layers().unwrap().base.same_storage(&base));
        assert!(editor
            .layers()
            .unwrap()
            .annotations
            .as_ref()
            .unwrap()
            .same_storage(&overlay));
        assert!(editor.undo());
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&base));
    }
}
