//! Closed windows revoke clipboard and drag publication, but admitted file
//! exports still own their write and report errors without targeting a dead HWND.
use crate::desktop::{copy_job, drag_prepare, export_job, State};

#[derive(Default)]
pub(super) struct Retired {
    jobs: Vec<Job>,
    pub archive: crate::desktop::export_image_autosave::Shared,
}

enum Job {
    Copy(copy_job::Job),
    Drag(drag_prepare::Job),
    Export(export_job::ExportJob),
}

impl Job {
    fn finished(&self) -> bool {
        match self {
            Self::Copy(job) => job.is_finished(),
            Self::Drag(job) => job.is_finished(),
            Self::Export(job) => job.is_finished(),
        }
    }

    fn complete(self) {
        if let Self::Export(job) = self {
            if let Err(error) = job.complete() {
                eprintln!("native-retired-export: {error}");
            }
        }
    }
}

impl Retired {
    pub(super) fn take_from(&mut self, state: &State) {
        if let Some(copy) = state.copy_job.take() {
            copy.cancel();
            self.jobs.push(Job::Copy(copy));
        }
        if let Some(drag) = state.drag.retire_preparation() {
            self.jobs.push(Job::Drag(drag));
        }
        if let Some(export) = state.export.take() {
            self.jobs.push(Job::Export(export));
        }
    }

    pub(super) fn reap(&mut self) {
        self.archive.borrow_mut().pump(false);
        let mut index = 0;
        while index < self.jobs.len() {
            if self.jobs[index].finished() {
                self.jobs.swap_remove(index).complete();
            } else {
                index += 1;
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.jobs.is_empty() && self.archive.try_borrow().is_ok_and(|owner| !owner.busy())
    }

    pub(super) fn shutdown(&mut self) {
        for job in self.jobs.drain(..) {
            job.complete();
        }
        let mut owner = self.archive.borrow_mut();
        owner.pump(true);
        if let Some(error) = owner.error.take() {
            eprintln!("native-archive-retired: {error}");
        }
    }
}
