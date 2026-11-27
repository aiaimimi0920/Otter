//! A modal owns one immutable source, one running encoder and one latest request.
//! Cancelled encoders never touch windows or files; the permit lives until worker exit.
use crate::desktop::export::codec::{Format, Source};
use crate::pixels::Pixels;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

// A closed modal can outlive its worker's cancellation point inside a codec.
// Keep ownership with the thread, not the dialog; a new modal waits without
// consuming another general output permit or displaying the previous result.
static ENCODING: AtomicBool = AtomicBool::new(false);
struct Slot;
impl Slot {
    fn acquire() -> Option<Self> {
        ENCODING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ENCODING.store(false, Ordering::Release);
    }
}

pub(super) struct Preview {
    source: Arc<Source>,
    pub desired: Format,
    job: Option<(Format, Arc<AtomicBool>, JoinHandle<Result<Pixels, String>>)>,
    pub result: Option<Result<Pixels, String>>,
}
impl Preview {
    pub fn new(source: Source) -> Self {
        Self {
            source: Arc::new(source),
            desired: Format::Png,
            job: None,
            result: None,
        }
    }
    pub fn request(&mut self, next: Format) {
        if self.desired != next {
            self.desired = next;
            self.result = None;
            if let Some((_, stop, _)) = &self.job {
                stop.store(true, Ordering::Release);
            }
        }
    }
    /// No waiting in the modal loop. A request superseded and then reinstated
    /// still starts a fresh job: its cancelled predecessor cannot publish.
    pub fn poll(&mut self) -> bool {
        if self
            .job
            .as_ref()
            .is_some_and(|(_, _, job)| job.is_finished())
        {
            let (fmt, stop, job) = self.job.take().unwrap();
            let value = job
                .join()
                .unwrap_or_else(|_| Err("Image encoder stopped".into()));
            if self.desired == fmt && !stop.load(Ordering::Acquire) {
                self.result = Some(value);
            }
        }
        if self.job.is_none() && self.result.is_none() {
            let Some(slot) = Slot::acquire() else {
                return true;
            };
            let permit = crate::edit_worker_budget::Permit::acquire_output();
            let Some(permit) = permit else {
                self.result = Some(Err("Output workers busy".into()));
                return false;
            };
            let source = self.source.clone();
            let fmt = self.desired;
            let stop = Arc::new(AtomicBool::new(false));
            let flag = stop.clone();
            match std::thread::Builder::new()
                .name("otter-export-preview".into())
                .spawn(move || {
                    let _slot = slot;
                    let _permit = permit;
                    source.encode(fmt, &flag).map_err(|e| e.to_string())
                }) {
                Ok(job) => self.job = Some((fmt, stop, job)),
                Err(e) => self.result = Some(Err(e.to_string())),
            }
        }
        self.job.is_some()
    }
}

#[cfg(test)]
#[path = "export_preview_tests.rs"]
mod tests;
impl Drop for Preview {
    fn drop(&mut self) {
        if let Some((_, stop, _)) = &self.job {
            stop.store(true, Ordering::Release);
        }
    }
}
