//! Closed windows revoke clipboard and drag publication, but admitted file
//! exports still own their write and report errors without targeting a dead HWND.
use crate::desktop::{copy_job, drag_prepare, export_job, State};

#[derive(Default)]
pub(super) struct Retired(Vec<Job>);

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
            self.0.push(Job::Copy(copy));
        }
        if let Some(drag) = state.drag.retire_preparation() {
            self.0.push(Job::Drag(drag));
        }
        if let Some(export) = state.export.take() {
            self.0.push(Job::Export(export));
        }
    }

    pub(super) fn reap(&mut self) {
        let mut index = 0;
        while index < self.0.len() {
            if self.0[index].finished() {
                self.0.swap_remove(index).complete();
            } else {
                index += 1;
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn shutdown(&mut self) {
        for job in self.0.drain(..) {
            job.complete();
        }
    }
}
