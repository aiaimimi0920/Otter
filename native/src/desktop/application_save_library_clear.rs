//! Clearing reuses the original library-update worker, cancellation, and Receipt publication; no disk-deletion task is created.
use super::*;
impl Job {
    pub fn start_library_clear(
        source: crate::session_source::Source,
        kind: crate::session_library::Kind,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_library(notify, move |cancel| {
            crate::session_library::remove::clear(&source, kind, cancel)
        })
    }
}
