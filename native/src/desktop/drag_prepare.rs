//! Single-task owner for drag-out snapshots; no HWND/clipboard operations, and unpublished staging is reclaimed on cancellation/Drop.
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

/// The UI freezes these values before startup; pixels/editor snapshots use existing shared storage, with no worker reads back into windows.
pub(super) struct Input {
    width: u32,
    height: u32,
    pixels: crate::pixels::Pixels,
    editor: crate::editor::Editor,
    opacity: f32,
    style: super::export::style::Snapshot,
    root: PathBuf,
    stem: String,
    policy: crate::cache_policy::Policy,
}
impl Input {
    pub(super) fn capture(
        canvas: &super::canvas::Canvas,
        frame: Option<&super::display_frame::Frame>,
        root: PathBuf,
        stem: String,
        policy: crate::cache_policy::Policy,
    ) -> Option<Self> {
        // Production accepts only editing snapshots with isolated history/drafts; transfer directly when starting workers.
        let (image, editor) = super::display_snapshot::capture(canvas, frame)?;
        Some(Self {
            width: image.width,
            height: image.height,
            pixels: image.pixels,
            editor,
            opacity: canvas.normal_opacity,
            style: super::export::style::Snapshot::capture_with_frame(canvas, frame),
            root,
            stem,
            policy,
        })
    }
}

pub struct Ready {
    cache: crate::clipboard_cache::Prepared,
    pub preview: super::drag_preview::Preview,
}
impl Ready {
    pub fn commit(self, cancel: &AtomicBool) -> std::io::Result<PathBuf> {
        self.cache.commit(cancel)
    }
    pub(super) fn publish(
        self,
        cancel: &AtomicBool,
    ) -> std::io::Result<(PathBuf, super::drag_preview::Preview, std::fs::File)> {
        let (path, lock) = self.cache.commit_drag(cancel)?;
        Ok((path, self.preview, lock))
    }
}
type Output = Result<Option<Ready>, String>;
pub struct Job {
    _permit: crate::edit_worker_budget::Permit<'static>,
    worker: Option<JoinHandle<Output>>,
    cancel: Arc<AtomicBool>,
}
impl Job {
    pub(super) fn start(input: Input) -> Result<Self, String> {
        validate(&input)?;
        let permit = crate::edit_worker_budget::Permit::acquire_output()
            .ok_or("Native output worker limit reached")?;
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("otter-drag-prepare".into())
            .spawn(move || prepare(input, &signal))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            _permit: permit,
            worker: Some(worker),
            cancel,
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
    #[cfg(test)]
    pub(super) fn from_test_worker(worker: JoinHandle<Output>, cancel: Arc<AtomicBool>) -> Self {
        Self {
            _permit: crate::edit_worker_budget::Permit::acquire_output().unwrap(),
            worker: Some(worker),
            cancel,
        }
    }
    pub fn is_finished(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(|worker| worker.is_finished())
    }
    /// Consume completed tasks only; callers must revalidate gesture, window, and document revision before deciding whether to publish staging.
    pub fn take_ready(&mut self) -> Option<Output> {
        if !self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return None;
        }
        let result = self
            .worker
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| Err("Native drag preparation worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Ok(None)
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // WIC encoding is uninterruptible, so exit must join; leave neither background threads nor completed staging holding locks.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn validate(input: &Input) -> Result<(), String> {
    if input.editor.draft().is_some() {
        return Err("Cannot export an active native drag draft".into());
    }
    if crate::viewport::checked_image_bytes(input.width, input.height) != Some(input.pixels.len()) {
        return Err("Invalid native drag image dimensions".into());
    }
    if !input.opacity.is_finite() || !(0.0..=1.0).contains(&input.opacity) {
        return Err("Invalid native drag opacity".into());
    }
    input.policy.validate().map_err(str::to_owned)?;
    Ok(())
}

fn prepare(input: Input, cancel: &AtomicBool) -> Output {
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    let image = super::image::Image {
        width: input.width,
        height: input.height,
        pixels: input.pixels,
    };
    let (png, preview) = input
        .style
        .png(&image, &input.editor, input.opacity)
        .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    let staged = crate::clipboard_cache::stage_with_policy(
        &input.root,
        &input.stem,
        png,
        cancel,
        input.policy,
    )
    .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Acquire) {
        Ok(None)
    } else {
        Ok(Some(Ready {
            cache: staged,
            preview,
        }))
    }
}

#[cfg(test)]
#[path = "drag_prepare_tests.rs"]
mod tests;
