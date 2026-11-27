//! Closed windows give up only their workers, never the HWND or State.
use super::*;
use crate::{blur_job, desktop, erase_job, flip_job, resize_job};
use std::time::{Duration, Instant};
#[path = "application_output_retirement.rs"]
mod output;

#[derive(Default)]
pub(super) struct Retired {
    raster: Vec<Jobs>,
    output: output::Retired,
    opens: Vec<desktop::open_job::OpenWorker>,
}

struct Jobs {
    blur: blur_job::Job,
    erase: erase_job::Job,
    flip: flip_job::Job,
    resize: resize_job::Job,
    rasterize: Option<desktop::rasterize_input::Job>,
    crop: Option<desktop::crop_apply::Job>,
}

impl Application {
    pub(in crate::desktop) fn bind_archive_profile(&self, root: Option<&std::path::Path>) {
        self.retired_edits.archive().borrow_mut().bind_profile(root);
    }
    pub(super) fn reap_retired_edits(&mut self) {
        let waiting = (self.exit_requested.get() || self.checkpoint_exit_waiting())
            && !self.retired_edits.outputs_settled();
        self.retired_edits.reap();
        if waiting && self.retired_edits.outputs_settled() {
            // Exit is checked before retirement in the pump. The last worker
            // also removes its deadline; explicitly wake the next exit turn.
            self.execution_wake.callback()();
        }
    }
}

impl Jobs {
    fn busy(&self) -> bool {
        self.blur.is_busy()
            || self.erase.is_busy()
            || self.flip.is_busy()
            || self.resize.is_busy()
            || self.rasterize.is_some()
            || self.crop.is_some()
    }

    fn reap(&mut self) {
        // A cancelled blur or erase can return None even after joining.
        // Capacity is released by actual owner state, not by a result.
        let _ = self.blur.poll();
        let _ = self.erase.poll();
        let _ = self.flip.poll();
        let _ = self.resize.poll();
        if self.rasterize.as_ref().is_some_and(|job| job.is_finished()) {
            self.rasterize = None;
        }
        if self.crop.as_ref().is_some_and(|job| job.is_finished()) {
            self.crop = None;
        }
    }
}

impl Retired {
    pub(super) fn with_archive(archive: desktop::export_image_autosave::Shared) -> Self {
        let mut result = Self::default();
        result.output.archive = archive;
        result
    }
    pub(super) fn archive(&self) -> &desktop::export_image_autosave::Shared {
        &self.output.archive
    }
    pub(super) fn outputs_settled(&self) -> bool {
        self.output.is_empty()
    }
    pub(super) fn take_from(&mut self, state: &State) {
        self.output.take_from(state);
        if let Some(worker) = desktop::open_job::retire(state) {
            if !worker.is_finished() {
                self.opens.push(worker);
            }
        }
        let mut jobs = Jobs {
            blur: state.blur_job.take(),
            erase: state.erase_job.take(),
            flip: state.flip_job.take(),
            resize: state.resize_job.take(),
            rasterize: state.rasterize_job.take(),
            crop: state.crop_job.take(),
        };
        jobs.blur.cancel(); // Also drops the pending replacement.
        jobs.erase.cancel();
        jobs.flip.cancel();
        jobs.resize.cancel();
        if let Some(job) = &jobs.rasterize {
            job.cancel();
        }
        if let Some(job) = &jobs.crop {
            job.cancel();
        }
        jobs.reap();
        if jobs.busy() {
            // Each nonempty entry owns at least one shared edit worker permit.
            self.raster.push(jobs);
        }
    }

    pub(super) fn reap(&mut self) {
        self.output.reap();
        self.opens.retain(|worker| !worker.is_finished());
        self.raster.retain_mut(|jobs| {
            jobs.reap();
            jobs.busy()
        });
    }

    pub(super) fn deadline(&self) -> Option<Instant> {
        // The destroyed HWND's timer can no longer signal completion.
        (!self.raster.is_empty() || !self.output.is_empty() || !self.opens.is_empty())
            .then(|| Instant::now() + Duration::from_millis(15))
    }

    pub(super) fn shutdown(&mut self) {
        self.output.shutdown();
        self.opens.clear();
        self.raster.clear(); // Only final process shutdown may wait for the workers.
    }
}

#[cfg(test)]
#[path = "application_edit_retirement_tests.rs"]
mod tests;
